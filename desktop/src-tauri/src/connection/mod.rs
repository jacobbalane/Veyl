//! Discovery remains active while one selected phone reconnects in the background.
mod worker;
use crate::{
    address,
    discovery::{DiscoveryEvent, NearbyTable},
    engine_host::{EngineHost, EngineSettings, HostOptions},
    events::*,
    pairings::{self, PairingStore},
    settings::Settings,
    source::PhoneSource,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot};
use veyl_link::{
    flow::Reconnect,
    messages::{Control, Ready},
    pairing::Identity,
};
use worker::{Event, Mode, Plan, Worker};

pub type Observer = Arc<dyn Fn(Notice) + Send + Sync>;
pub enum Command {
    Pair(PhoneTarget),
    Confirm { session_id: String, matches: bool },
    Connect(PhoneTarget),
    Disconnect,
    Forget(String),
    RequestStart,
    SetActive(bool, oneshot::Sender<()>),
    Shutdown(oneshot::Sender<()>),
}
#[derive(Clone)]
pub struct ConnectionManager {
    commands: mpsc::Sender<Command>,
    view: Arc<Mutex<LinkSnapshot>>,
    engine: Arc<EngineHost>,
    source: PhoneSource,
}
impl ConnectionManager {
    #[cfg(test)]
    pub(crate) fn isolated() -> (Self, mpsc::Receiver<Command>) {
        let (commands, receiver) = mpsc::channel(32);
        let (engine, source) = EngineHost::start();
        (
            Self {
                commands,
                view: Arc::new(Mutex::new(LinkSnapshot::default())),
                engine: Arc::new(engine),
                source,
            },
            receiver,
        )
    }
    pub fn start(
        settings: Arc<Mutex<Settings>>,
        directory: &Path,
        observer: Observer,
        recorder: crate::recorder::Recorder,
    ) -> io::Result<Self> {
        let pc_id = pairings::id_bytes(&settings.lock().unwrap().pc_id)?;
        let store = PairingStore::load(&directory.join("pairings.json"), pc_id)?;
        let (commands, receiver) = mpsc::channel(32);
        let view = Arc::new(Mutex::new(LinkSnapshot::default()));
        let (outgoing, outgoing_rx) = mpsc::channel(8);
        let engine_settings = EngineSettings::from_settings(&settings.lock().unwrap());
        let (engine, source) = EngineHost::launch(HostOptions {
            recorder: Some(recorder),
            settings: engine_settings,
            observer: Some(observer.clone()),
            phone: Some(outgoing),
        });
        let engine = Arc::new(engine);
        let active = settings.lock().unwrap().source == "phone";
        let actor = Actor {
            settings,
            settings_path: directory.join("settings.json"),
            store,
            view: view.clone(),
            observer,
            nearby: NearbyTable::default(),
            source: source.clone(),
            active,
            worker: None,
            generation: 0,
            goal: None,
            paused: false,
            reconnect: Reconnect::default(),
            retry_at: None,
            origin: Instant::now(),
        };
        tauri::async_runtime::spawn(actor.run(receiver, outgoing_rx));
        Ok(Self {
            commands,
            view,
            engine,
            source,
        })
    }
    pub(crate) fn source(&self) -> PhoneSource {
        self.source.clone()
    }
    pub(crate) fn engine(&self) -> &EngineHost {
        &self.engine
    }
    pub async fn set_active(&self, active: bool) -> Result<(), String> {
        let (done, wait) = oneshot::channel();
        self.commands
            .send(Command::SetActive(active, done))
            .await
            .map_err(|_| "Connection manager stopped.")?;
        wait.await.map_err(|_| "Connection manager stopped.".into())
    }
    pub fn send(&self, command: Command) -> Result<(), String> {
        self.commands
            .try_send(command)
            .map_err(|_| "Veyl's connection manager is busy. Try again.".into())
    }
    pub fn snapshot(&self) -> LinkSnapshot {
        self.view
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
    pub fn diagnostics(&self) -> Diagnostics {
        let mut link = self.snapshot();
        if let Some(pairing) = link.pairing.as_mut() {
            pairing.code = None;
        }
        Diagnostics {
            link,
            levels: self.engine.levels(),
            source: None,
        }
    }
    pub async fn shutdown(&self) {
        let (done, wait) = oneshot::channel();
        let _ = tokio::time::timeout(Duration::from_secs(2), async {
            if self.commands.send(Command::Shutdown(done)).await.is_ok() {
                let _ = wait.await;
            }
        })
        .await;
    }
}
#[derive(Clone)]
struct Goal {
    phone_id: Option<String>,
    name: String,
    address: String,
}
struct Actor {
    settings: Arc<Mutex<Settings>>,
    settings_path: PathBuf,
    store: PairingStore,
    view: Arc<Mutex<LinkSnapshot>>,
    observer: Observer,
    nearby: NearbyTable,
    source: PhoneSource,
    active: bool,
    worker: Option<Worker>,
    generation: u64,
    goal: Option<Goal>,
    paused: bool,
    reconnect: Reconnect,
    retry_at: Option<Instant>,
    origin: Instant,
}
impl Actor {
    async fn run(
        mut self,
        mut commands: mpsc::Receiver<Command>,
        mut outgoing: mpsc::Receiver<Control>,
    ) {
        let (events, event_rx) = mpsc::channel(256);
        let mut event_rx = event_rx;
        let (dns, dns_rx) = mpsc::channel(128);
        let mut dns_rx = dns_rx;
        #[cfg(windows)]
        let mut browse = if self.active {
            crate::discovery::NativeRequest::browse(dns.clone()).ok()
        } else {
            None
        };
        #[cfg(windows)]
        let mut resolves =
            std::collections::BTreeMap::<String, (crate::discovery::NativeRequest, Instant)>::new();
        let mut last_browse = Instant::now();
        let mut ticks = tokio::time::interval(Duration::from_millis(250));
        let mut shutdown = None;
        let mut engine_open = true;
        self.publish_phones();
        loop {
            tokio::select! {
                command=commands.recv()=> {
                    let Some(command)=command else {break};
                    if let Command::Shutdown(done)=command {shutdown=Some(done);break}
                    if let Command::SetActive(active, done) = command {
                        if self.active != active {
                            self.stop("user_quit").await;
                            self.active = active;
                            self.goal = None; self.retry_at = None; self.paused = false;
                            self.nearby = NearbyTable::default();
                            { let mut view = self.view.lock().unwrap(); view.pairing = None; view.connection = ConnectionState::default(); view.phone_stats = PhoneStats::default(); }
                            #[cfg(windows)]
                            { resolves.clear(); browse.take(); if active { browse = crate::discovery::NativeRequest::browse(dns.clone()).ok(); } }
                            while dns_rx.try_recv().is_ok() {}
                            self.publish_phones(); self.connection(if active { "browsing" } else { "idle" }, None);
                            last_browse = Instant::now();
                        }
                        let _ = done.send(()); continue;
                    }
                    self.command(command,&events).await;
                },
                event=event_rx.recv()=> {if let Some(event)=event {self.event(event).await;}},
                // The engine's meter and gestures go to the phone of the live session only.
                message=outgoing.recv(),if engine_open=> {match (message,&self.worker) {(None,_)=>engine_open=false,(Some(message),Some(worker)) if self.view.lock().unwrap().connection.state=="connected"=> {let _=worker.controls.try_send(message);},_=>{}}},
                event=dns_rx.recv()=> {
                    if !self.active { continue; }
                    match event {
                        Some(DiscoveryEvent::Resolved(service))=> {
                            let id=service.properties.get("id").map(|id|id.to_ascii_lowercase());let address=service.address.to_string();
                            if self.nearby.update(service,Instant::now()) {self.publish_phones();}
                            let accepted=self.nearby.phones().iter().any(|phone|Some(&phone.id)==id.as_ref() && phone.address==address);
                            if accepted {if let Some(goal)=self.goal.as_mut() {if goal.phone_id.as_ref().and_then(|id|pairings::short_id(id).ok())==id {goal.address=address;if self.worker.is_none() {self.retry_at=Some(Instant::now());}}}}
                        },
                        Some(DiscoveryEvent::Instance {name,gone})=> {
                            if gone && self.nearby.remove_instance(&name) {self.publish_phones();}
                            #[cfg(windows)]
                            if !gone && resolves.len()<32 && !resolves.contains_key(&name) {
                                if let Ok(request)=crate::discovery::NativeRequest::resolve(&name,dns.clone()) {resolves.insert(name,(request,Instant::now()));}
                            }
                        },
                        Some(DiscoveryEvent::Failed(_))=>tracing::debug!("System phone discovery reported an error."),
                        None=>{},
                    }
                },
                _=ticks.tick()=> {
                    if self.nearby.expire(Instant::now()) {self.publish_phones();}
                    #[cfg(windows)]
                    resolves.retain(|_,(request,started)|!request.is_done() && started.elapsed()<Duration::from_secs(5));
                    if self.active && last_browse.elapsed()>=Duration::from_secs(30) {
                        #[cfg(windows)]
                        {browse.take();browse=crate::discovery::NativeRequest::browse(dns.clone()).ok();}
                        last_browse=Instant::now();
                    }
                    if self.active && self.worker.is_none() {
                        if self.retry_at.is_some_and(|at|Instant::now()>=at) {self.launch(false,&events).await;}
                        else if self.goal.is_none() && !self.paused {self.auto_connect(&events).await;}
                    }
                },
            }
        }
        self.stop("user_quit").await;
        #[cfg(windows)]
        {
            drop(resolves);
            drop(browse);
        }
        if let Some(done) = shutdown {
            let _ = done.send(());
        }
    }
    fn identity(&self) -> io::Result<Identity> {
        let settings = self.settings.lock().unwrap();
        Ok(Identity {
            id: pairings::id_bytes(&settings.pc_id)?,
            name: settings.pc_name.clone(),
            model: String::new(),
            app: format!("veyl-desktop/{}", env!("CARGO_PKG_VERSION")),
        })
    }
    fn publish_phones(&self) {
        let paired = self.store.list();
        let nearby = self
            .nearby
            .phones()
            .into_iter()
            .filter(|phone| {
                !paired.iter().any(|saved| {
                    pairings::short_id(&saved.phone_id).ok().as_deref() == Some(phone.id.as_str())
                })
            })
            .collect();
        let phones = PhonesView { paired, nearby };
        self.view.lock().unwrap().phones = phones.clone();
        (self.observer)(Notice::Phones(phones));
    }
    fn connection(&self, state: &str, error: Option<&str>) {
        let next = {
            let mut view = self.view.lock().unwrap();
            view.connection.state = state.into();
            view.connection.error = error.map(str::to_owned);
            view.connection.clone()
        };
        (self.observer)(Notice::Connection(next));
    }
    fn pairing(
        &self,
        stage: &str,
        code: Option<String>,
        name: Option<String>,
        error: Option<&str>,
    ) {
        let next = {
            let mut view = self.view.lock().unwrap();
            let Some(pairing) = view.pairing.as_mut() else {
                return;
            };
            pairing.stage = stage.into();
            if stage != "waiting_phone" {
                pairing.code = code;
            }
            if let Some(name) = name {
                pairing.phone_name = name;
            }
            pairing.error = error.map(str::to_owned);
            pairing.clone()
        };
        (self.observer)(Notice::Pairing(next));
    }
    fn target(&self, target: PhoneTarget) -> io::Result<Goal> {
        match target {
            PhoneTarget::Address { address: input } => {
                address::parse(&input)?;
                let address = input.trim().to_owned();
                let saved = self
                    .store
                    .list()
                    .into_iter()
                    .find(|phone| phone.last_address == address);
                Ok(Goal {
                    phone_id: saved.as_ref().map(|phone| phone.phone_id.clone()),
                    name: saved
                        .map(|phone| phone.phone_name)
                        .unwrap_or_else(|| address.clone()),
                    address,
                })
            }
            PhoneTarget::Saved { phone_id } => {
                let phone = self
                    .store
                    .list()
                    .into_iter()
                    .find(|phone| phone.phone_id == phone_id)
                    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Phone not found."))?;
                Ok(Goal {
                    phone_id: Some(phone_id),
                    name: phone.phone_name,
                    address: phone.last_address,
                })
            }
            PhoneTarget::Nearby { nearby_id } => {
                let phone = self
                    .nearby
                    .phones()
                    .into_iter()
                    .find(|phone| phone.id == nearby_id)
                    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Phone not found."))?;
                let saved = self.store.list().into_iter().find(|saved| {
                    pairings::short_id(&saved.phone_id).ok().as_deref() == Some(nearby_id.as_str())
                });
                Ok(Goal {
                    phone_id: saved.map(|phone| phone.phone_id),
                    name: phone.name,
                    address: phone.address,
                })
            }
        }
    }
    async fn command(&mut self, command: Command, events: &mpsc::Sender<Event>) {
        if !self.active && !matches!(&command, Command::Forget(_)) {
            return;
        }
        let pair = matches!(&command, Command::Pair(_));
        match command {
            Command::Pair(target) | Command::Connect(target) => match self.target(target) {
                Ok(goal) => {
                    self.stop("user_quit").await;
                    self.goal = Some(goal);
                    self.paused = false;
                    self.reconnect.connected();
                    self.retry_at = None;
                    self.launch(pair, events).await;
                }
                Err(_) => self.connection("browsing", Some("invalid_address")),
            },
            Command::Confirm {
                session_id,
                matches,
            } => {
                let current = self.view.lock().unwrap().pairing.clone();
                if current.as_ref().is_some_and(|current| {
                    current.session_id == session_id
                        && ((!matches
                            && matches!(
                                current.stage.as_str(),
                                "connecting" | "compare" | "waiting_phone"
                            ))
                            || (matches && current.stage == "compare"))
                }) {
                    if let Some(worker) = &self.worker {
                        let _ = worker.confirmations.try_send(matches);
                    }
                    if matches {
                        self.pairing("waiting_phone", None, None, None);
                    } else {
                        self.pairing("failed", None, None, Some("user_cancelled"));
                        self.stop("user_quit").await;
                        self.goal = None;
                        self.paused = true;
                        self.connection("browsing", None);
                    }
                }
            }
            Command::Disconnect => {
                self.stop("user_quit").await;
                self.goal = None;
                self.retry_at = None;
                self.paused = true;
                self.pairing("failed", None, None, Some("user_cancelled"));
                self.connection("browsing", None);
            }
            Command::Forget(id) => match self.store.forget(&id) {
                Ok(()) => {
                    if self.goal.as_ref().and_then(|goal| goal.phone_id.as_ref()) == Some(&id) {
                        self.stop("forgotten").await;
                        self.goal = None;
                        self.retry_at = None;
                        self.paused = true;
                        self.connection("browsing", None);
                    }
                    self.publish_phones();
                }
                Err(_) => self.connection("browsing", Some("pair_store_write_failed")),
            },
            Command::RequestStart => {
                if let Some(worker) = &self.worker {
                    if self.view.lock().unwrap().connection.state == "connected" {
                        let _ = worker.controls.try_send(Control::RequestStart);
                    }
                }
            }
            Command::Shutdown(_) | Command::SetActive(..) => {}
        }
    }
    async fn stop(&mut self, reason: &'static str) {
        self.source.disconnect_now(self.generation);
        self.generation = self.source.next_generation();
        if let Some(worker) = self.worker.take() {
            worker.stop(reason).await;
        }
    }
    async fn launch(&mut self, pair: bool, events: &mpsc::Sender<Event>) {
        if !self.active || self.settings.lock().unwrap().source != "phone" {
            self.connection("idle", None);
            self.retry_at = None;
            return;
        }
        let Some(goal) = self.goal.clone() else {
            return;
        };
        self.generation = self.source.next_generation();
        let generation = self.generation;
        let mode = if pair {
            Mode::Pair
        } else if let Some(id) = &goal.phone_id {
            match self.store.find(id) {
                Ok(Some(peer)) => Mode::Session(peer),
                _ => {
                    self.connection("browsing", Some("pair_store_read_failed"));
                    self.goal = None;
                    self.retry_at = None;
                    self.paused = true;
                    return;
                }
            }
        } else {
            Mode::Probe
        };
        let identity = match self.identity() {
            Ok(identity) => identity,
            Err(_) => {
                self.connection("browsing", Some("settings"));
                return;
            }
        };
        {
            let mut view = self.view.lock().unwrap();
            view.connection.phone_id = goal.phone_id.clone();
            view.connection.phone_name = Some(goal.name.clone());
            view.connection.address = Some(goal.address.clone());
            view.connection.capture = None;
            view.phone_stats = PhoneStats::default();
        }
        if pair {
            let mut id = [0; 16];
            if getrandom::fill(&mut id).is_err() {
                self.connection("browsing", Some("internal"));
                return;
            }
            self.view.lock().unwrap().pairing = Some(PairingEvent {
                session_id: URL_SAFE_NO_PAD.encode(id),
                phone_name: goal.name.clone(),
                code: None,
                stage: "connecting".into(),
                error: None,
            });
            self.pairing("connecting", None, None, None);
            self.connection("pairing", None);
        } else {
            self.view.lock().unwrap().pairing = None;
            self.connection("connecting", None);
        }
        self.retry_at = None;
        self.worker = Some(Worker::start(
            Plan {
                generation,
                address: goal.address,
                identity,
                mode,
            },
            events.clone(),
            self.source.clone(),
            self.origin,
        ));
    }
    async fn auto_connect(&mut self, events: &mpsc::Sender<Event>) {
        if self.settings.lock().unwrap().source != "phone" {
            return;
        }
        let phones = self.nearby.phones();
        let saved = self.store.list();
        let preferred = self.settings.lock().unwrap().last_phone_id.clone();
        let mut candidates: Vec<_> = saved
            .into_iter()
            .filter_map(|saved| {
                phones
                    .iter()
                    .find(|phone| {
                        pairings::short_id(&saved.phone_id).ok().as_deref()
                            == Some(phone.id.as_str())
                    })
                    .map(|nearby| Goal {
                        phone_id: Some(saved.phone_id.clone()),
                        name: saved.phone_name,
                        address: nearby.address.clone(),
                    })
            })
            .collect();
        candidates.sort_by_key(|goal| goal.phone_id != preferred);
        if self.origin.elapsed() < Duration::from_secs(2)
            && candidates
                .first()
                .is_some_and(|goal| goal.phone_id != preferred)
        {
            return;
        }
        if let Some(goal) = candidates.into_iter().next() {
            self.goal = Some(goal);
            self.launch(false, events).await;
        }
    }
    async fn event(&mut self, event: Event) {
        let generation = match &event {
            Event::Stage(g, _)
            | Event::Compare(g, ..)
            | Event::Paired(g, ..)
            | Event::Ready(g, ..)
            | Event::Info(g, ..)
            | Event::Latency(g, ..)
            | Event::Probed(g, ..)
            | Event::Ended(g, ..) => *g,
        };
        if !self.active || generation != self.generation {
            return;
        }
        match event {
            Event::Stage(_, state) => {
                if self.view.lock().unwrap().connection.state != "pairing" {
                    self.connection(state, None);
                }
            }
            Event::Compare(_, name, code) => self.pairing("compare", Some(code), Some(name), None),
            Event::Paired(_, peer, address) => match self.store.remember(&peer, address) {
                Ok(()) => {
                    self.pairing("done", None, Some(peer.peer.name.clone()), None);
                    self.stop("user_quit").await;
                    self.goal = Some(Goal {
                        phone_id: Some(URL_SAFE_NO_PAD.encode(peer.peer.id)),
                        name: peer.peer.name.clone(),
                        address: address.to_string(),
                    });
                    self.publish_phones();
                    self.retry_at = Some(Instant::now() + Duration::from_millis(250));
                }
                Err(_) => {
                    self.pairing("failed", None, None, Some("pair_store_write_failed"));
                    self.goal = None;
                    self.paused = true;
                }
            },
            Event::Probed(_, id, address) => {
                self.stop("user_quit").await;
                if let Some(phone) = self
                    .store
                    .list()
                    .into_iter()
                    .find(|phone| phone.phone_id == id)
                {
                    self.goal = Some(Goal {
                        phone_id: Some(id),
                        name: phone.phone_name,
                        address: address.to_string(),
                    });
                    self.retry_at = Some(Instant::now() + Duration::from_millis(250));
                } else {
                    self.goal = None;
                    self.paused = true;
                    self.connection("browsing", Some("not_paired"));
                }
            }
            Event::Ready(_, Ready::Phone { device, capture }, address) => {
                self.reconnect.connected();
                let id = self.goal.as_ref().and_then(|goal| goal.phone_id.clone());
                if let Some(id) = id {
                    if self
                        .store
                        .seen(&id, address, Some(&device.phone_name))
                        .is_err()
                    {
                        tracing::warn!("Paired phone metadata could not be saved.");
                    }
                    let mut settings = self.settings.lock().unwrap();
                    settings.last_phone_id = Some(id);
                    if settings.save(&self.settings_path).is_err() {
                        tracing::warn!("Connection preference could not be saved.");
                    }
                }
                {
                    let mut view = self.view.lock().unwrap();
                    view.connection.phone_name = Some(device.phone_name);
                    view.connection.capture = Some(capture.clone());
                    view.connection.attempt = 0;
                }
                self.connection("connected", None);
                (self.observer)(Notice::Capture(capture));
                self.publish_phones();
            }
            Event::Ready(_, _, _) => {}
            Event::Info(_, message) => match message {
                Control::CaptureState { capture } => {
                    self.view.lock().unwrap().connection.capture = Some(capture.clone());
                    (self.observer)(Notice::Capture(capture));
                    self.connection("connected", None);
                }
                Control::Stats {
                    queue_ms,
                    dropped_blocks,
                    media_volume,
                    battery_pct,
                    charging,
                    thermal,
                    cpu_pct,
                } => {
                    let mut view = self.view.lock().unwrap();
                    let rtt = view.phone_stats.rtt_ms;
                    view.phone_stats = PhoneStats {
                        queue_ms,
                        dropped_blocks,
                        media_volume: media_volume
                            .is_finite()
                            .then(|| media_volume.clamp(0.0, 1.0)),
                        battery_pct: Some(battery_pct.min(100)),
                        charging,
                        thermal: Some(thermal),
                        cpu_pct: cpu_pct.is_finite().then(|| cpu_pct.clamp(0.0, 100.0)),
                        rtt_ms: rtt,
                    };
                }
                Control::PhoneInfo { phone_name } if phone_name.len() <= 40 => {
                    if let Some(goal) = self.goal.as_mut() {
                        goal.name = phone_name.clone();
                        if let (Some(id), Ok(address)) = (&goal.phone_id, goal.address.parse()) {
                            let _ = self.store.seen(id, address, Some(&phone_name));
                        }
                    }
                    self.view.lock().unwrap().connection.phone_name = Some(phone_name);
                    self.publish_phones();
                    self.connection("connected", None);
                }
                _ => {}
            },
            Event::Latency(_, rtt) => self.view.lock().unwrap().phone_stats.rtt_ms = Some(rtt),
            Event::Ended(_, reason) => {
                if let Some(worker) = self.worker.take() {
                    worker.stop("user_quit").await;
                }
                if self
                    .view
                    .lock()
                    .unwrap()
                    .pairing
                    .as_ref()
                    .is_some_and(|pairing| !matches!(pairing.stage.as_str(), "done" | "failed"))
                {
                    self.pairing("failed", None, None, Some(reason));
                    self.goal = None;
                    self.paused = true;
                    self.connection("browsing", Some(reason));
                } else if reason == "not_paired" {
                    self.goal = None;
                    self.paused = true;
                    self.connection("browsing", Some("not_paired"));
                } else if self.goal.is_some() {
                    let now = self.origin.elapsed().as_millis() as u64;
                    let delay = self.reconnect.failed(now);
                    self.retry_at = Some(Instant::now() + Duration::from_millis(delay));
                    {
                        let mut view = self.view.lock().unwrap();
                        view.connection.attempt += 1;
                    }
                    self.connection(
                        if self.reconnect.unreachable(now) {
                            "unreachable"
                        } else {
                            "reconnecting"
                        },
                        Some(reason),
                    );
                } else {
                    self.connection("browsing", None);
                }
            }
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use veyl_link::{
        crypto::Secret,
        messages::{Capture, Device},
        pairing::{PairedPeer, Peer},
    };
    fn peer() -> PairedPeer {
        PairedPeer {
            peer: Peer {
                id: [2; 16],
                name: "Test phone".into(),
                model: "Test".into(),
            },
            link: Secret::new([3; 32]),
        }
    }
    fn actor(directory: &Path) -> Actor {
        let path = directory.join("settings.json");
        let settings = Settings::load_or_create(&path).unwrap();
        let id = pairings::id_bytes(&settings.pc_id).unwrap();
        let (_, source) = EngineHost::start();
        Actor {
            settings: Arc::new(Mutex::new(settings)),
            settings_path: path,
            store: PairingStore::load(&directory.join("pairings.json"), id).unwrap(),
            view: Arc::new(Mutex::new(LinkSnapshot::default())),
            observer: Arc::new(|_| {}),
            nearby: NearbyTable::default(),
            source,
            active: true,
            worker: None,
            generation: 7,
            goal: None,
            paused: false,
            reconnect: Reconnect::default(),
            retry_at: None,
            origin: Instant::now(),
        }
    }
    #[tokio::test]
    async fn stale_pairing_decisions_and_worker_results_cannot_replace_the_current_session() {
        let directory = tempfile::tempdir().unwrap();
        let mut actor = actor(directory.path());
        let (events, _) = mpsc::channel(16);
        actor.view.lock().unwrap().pairing = Some(PairingEvent {
            session_id: "current".into(),
            phone_name: "Test phone".into(),
            code: Some("123456".into()),
            stage: "compare".into(),
            error: None,
        });
        actor
            .command(
                Command::Confirm {
                    session_id: "stale".into(),
                    matches: true,
                },
                &events,
            )
            .await;
        assert_eq!(
            actor.view.lock().unwrap().pairing.as_ref().unwrap().stage,
            "compare"
        );
        actor
            .event(Event::Paired(6, peer(), "127.0.0.1:47820".parse().unwrap()))
            .await;
        assert!(actor.store.list().is_empty());
        actor
            .event(Event::Paired(7, peer(), "127.0.0.1:47820".parse().unwrap()))
            .await;
        assert_eq!(actor.store.list().len(), 1);
        assert_eq!(
            actor.view.lock().unwrap().pairing.as_ref().unwrap().stage,
            "done"
        );
        let make_ready = || Ready::Phone {
            device: Device {
                manufacturer: "Test".into(),
                model: "Test".into(),
                android_sdk: 26,
                app_version: "test".into(),
                phone_name: "Renamed phone".into(),
            },
            capture: Capture {
                state: "idle".into(),
                source: "capture".into(),
                route: "speaker".into(),
                reason: None,
            },
        };
        actor
            .event(Event::Ready(
                7,
                make_ready(),
                "127.0.0.1:47821".parse().unwrap(),
            ))
            .await;
        assert_ne!(actor.view.lock().unwrap().connection.state, "connected");
        actor
            .event(Event::Ready(
                actor.generation,
                make_ready(),
                "127.0.0.1:47821".parse().unwrap(),
            ))
            .await;
        assert_eq!(actor.view.lock().unwrap().connection.state, "connected");
        assert_eq!(actor.store.list()[0].phone_name, "Renamed phone");
        let id = URL_SAFE_NO_PAD.encode([2; 16]);
        assert_eq!(
            actor.settings.lock().unwrap().last_phone_id.as_ref(),
            Some(&id)
        );
        actor.command(Command::Forget(id), &events).await;
        assert!(actor.store.list().is_empty());
        assert!(actor.goal.is_none());
        assert!(actor.paused);
        assert_eq!(actor.view.lock().unwrap().connection.state, "browsing");
    }
    #[test]
    fn diagnostics_remove_comparison_codes_and_targets_reject_ambiguous_fields() {
        let (commands, _) = mpsc::channel(1);
        let (engine, source) = EngineHost::start();
        let snapshot = LinkSnapshot {
            pairing: Some(PairingEvent {
                session_id: "current".into(),
                phone_name: "Test phone".into(),
                code: Some("123456".into()),
                stage: "compare".into(),
                error: None,
            }),
            ..Default::default()
        };
        let manager = ConnectionManager {
            commands,
            view: Arc::new(Mutex::new(snapshot)),
            engine: Arc::new(engine),
            source,
        };
        assert!(manager.snapshot().pairing.unwrap().code.is_some());
        assert!(manager.diagnostics().link.pairing.unwrap().code.is_none());
        assert!(!serde_json::to_string(&manager.diagnostics())
            .unwrap()
            .contains("123456"));
        for value in [
            serde_json::json!({"address":"127.0.0.1:1","nearbyId":"abcd1234"}),
            serde_json::json!({"phoneId":"x","unexpected":true}),
            serde_json::json!({}),
        ] {
            assert!(serde_json::from_value::<PhoneTarget>(value).is_err());
        }
    }
}
