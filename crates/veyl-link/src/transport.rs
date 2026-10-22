//! Readers and writers retain partial frames across cancellation and run independently.
use crate::{
    crypto::SealedChannel,
    frames::validate,
    invalid,
    messages::{Control, Ready},
    pairing::{Identity, PairedPeer, Pairing, Peer},
    session::Session,
    Frame, MAX_FRAME,
};
use std::{collections::VecDeque, io, net::SocketAddrV4, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf},
    sync::mpsc,
    time::{self, Instant},
};

pub struct Receiver<R> {
    stream: R,
    header: [u8; 5],
    header_used: usize,
    body: Vec<u8>,
    body_used: usize,
    closed: bool,
}
impl<R: AsyncRead + Unpin> Receiver<R> {
    pub async fn receive(&mut self) -> io::Result<Frame> {
        let result = self.receive_inner().await;
        if result.is_err() {
            self.closed = true;
        }
        result
    }
    async fn receive_inner(&mut self) -> io::Result<Frame> {
        if self.closed {
            return Err(invalid("Connection closed."));
        }
        while self.header_used < 5 {
            let read = self
                .stream
                .read(&mut self.header[self.header_used..])
                .await?;
            if read == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            self.header_used += read;
        }
        if self.body.is_empty() && self.body_used == 0 {
            let length = u32::from_be_bytes(self.header[..4].try_into().unwrap()) as usize;
            if !(1..=MAX_FRAME).contains(&length) {
                return Err(invalid("Invalid frame length."));
            }
            validate(self.header[4], length - 1)?;
            self.body.resize(length - 1, 0);
        }
        while self.body_used < self.body.len() {
            let read = self.stream.read(&mut self.body[self.body_used..]).await?;
            if read == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            self.body_used += read;
        }
        let frame = Frame {
            kind: self.header[4],
            body: std::mem::take(&mut self.body),
        };
        self.header_used = 0;
        self.body_used = 0;
        Ok(frame)
    }
}
pub struct Sender<W> {
    stream: W,
    queue: VecDeque<Vec<u8>>,
    written: usize,
    closed: bool,
    dirty: bool,
}
impl<W: AsyncWrite + Unpin> Sender<W> {
    pub fn enqueue(&mut self, frame: &Frame) -> io::Result<()> {
        if self.closed {
            return Err(invalid("Connection closed."));
        }
        self.queue.push_back(frame.wire()?);
        self.dirty = true;
        Ok(())
    }
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty() && !self.dirty
    }
    pub async fn flush(&mut self) -> io::Result<()> {
        let result = self.flush_inner().await;
        if result.is_err() {
            self.closed = true;
        }
        result
    }
    async fn flush_inner(&mut self) -> io::Result<()> {
        if self.closed {
            return Err(invalid("Connection closed."));
        }
        while let Some(bytes) = self.queue.front() {
            while self.written < bytes.len() {
                let written = self.stream.write(&bytes[self.written..]).await?;
                if written == 0 {
                    return Err(io::ErrorKind::WriteZero.into());
                }
                self.written += written;
            }
            self.queue.pop_front();
            self.written = 0;
        }
        self.stream.flush().await?;
        self.dirty = false;
        Ok(())
    }
}
pub struct Connection<S> {
    pub receiver: Receiver<ReadHalf<S>>,
    pub sender: Sender<WriteHalf<S>>,
}
impl<S: AsyncRead + AsyncWrite + Unpin> Connection<S> {
    pub fn new(stream: S) -> Self {
        let (reader, writer) = tokio::io::split(stream);
        Self {
            receiver: Receiver {
                stream: reader,
                header: [0; 5],
                header_used: 0,
                body: vec![],
                body_used: 0,
                closed: false,
            },
            sender: Sender {
                stream: writer,
                queue: VecDeque::new(),
                written: 0,
                closed: false,
                dirty: false,
            },
        }
    }
    pub async fn receive(&mut self) -> io::Result<Frame> {
        self.receiver.receive().await
    }
    pub async fn send(&mut self, frame: &Frame) -> io::Result<()> {
        self.sender.enqueue(frame)?;
        self.sender.flush().await
    }
    pub async fn close(&mut self) {
        self.receiver.closed = true;
        self.sender.closed = true;
        let _ = time::timeout(Duration::from_millis(100), self.sender.stream.shutdown()).await;
    }
}
pub async fn connect(address: SocketAddrV4) -> io::Result<Connection<tokio::net::TcpStream>> {
    let stream = time::timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect(address),
    )
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Connection timed out."))??;
    stream.set_nodelay(true)?;
    Ok(Connection::new(stream))
}
pub async fn pair<S, F>(
    connection: &mut Connection<S>,
    identity: Identity,
    mut show_code: F,
    confirmations: &mut mpsc::Receiver<bool>,
) -> io::Result<PairedPeer>
where
    S: AsyncRead + AsyncWrite + Unpin,
    F: FnMut(&str),
{
    pair_with_progress(
        connection,
        identity,
        |code, _| show_code(code),
        confirmations,
    )
    .await
}
pub async fn pair_with_progress<S, F>(
    connection: &mut Connection<S>,
    identity: Identity,
    mut show_code: F,
    confirmations: &mut mpsc::Receiver<bool>,
) -> io::Result<PairedPeer>
where
    S: AsyncRead + AsyncWrite + Unpin,
    F: FnMut(&str, &Peer),
{
    let result = time::timeout(
        Duration::from_secs(150),
        pair_inner(connection, identity, &mut show_code, confirmations),
    )
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Pairing timed out."))
    .and_then(|result| result);
    if result.is_err() {
        connection.close().await;
    }
    result
}
async fn pair_inner<S, F>(
    connection: &mut Connection<S>,
    identity: Identity,
    show_code: &mut F,
    confirmations: &mut mpsc::Receiver<bool>,
) -> io::Result<PairedPeer>
where
    S: AsyncRead + AsyncWrite + Unpin,
    F: FnMut(&str, &Peer),
{
    let start = Instant::now();
    let (mut machine, hello) = Pairing::pc(identity, 0)?;
    connection.sender.enqueue(&hello)?;
    let mut shown = false;
    let mut confirmed = false;
    let mut abort_deadline = None;
    let mut ticks = time::interval(Duration::from_millis(100));
    loop {
        if connection.sender.is_empty() {
            if let Some(pairing) = machine.take_pairing() {
                return Ok(pairing);
            }
            if let Some(reason) = machine.failure() {
                return Err(crate::rejected(reason));
            }
        }
        if machine.failure().is_some() && abort_deadline.is_none() {
            abort_deadline = Some(Instant::now() + Duration::from_millis(100));
        }
        if !shown {
            if let Some(sas) = machine.sas() {
                show_code(
                    sas,
                    machine
                        .peer()
                        .ok_or_else(|| invalid("Pairing peer is missing."))?,
                );
                shown = true;
            }
        }
        let elapsed = || start.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let outbound = tokio::select! {
            frame = connection.receiver.receive(), if !machine.is_done() => {
                let frame = frame?;
                if machine.failure().is_none() { machine.receive(&frame, elapsed())? } else { vec![] }
            }
            decision = confirmations.recv(), if !machine.is_done() && machine.failure().is_none() => {
                if !decision.unwrap_or(false) { confirmed = true; machine.cancel()? }
                else if shown && !confirmed { confirmed = true; machine.confirm(true, elapsed())? }
                else { vec![] }
            }
            _ = time::sleep_until(abort_deadline.unwrap_or(start + Duration::from_secs(150))), if abort_deadline.is_some() => return Err(crate::rejected(machine.failure().unwrap_or("bad_message"))),
            _ = ticks.tick() => machine.tick(elapsed())?,
            result = connection.sender.flush(), if !connection.sender.is_empty() => { result?; vec![] }
        };
        for frame in outbound {
            connection.sender.enqueue(&frame)?;
        }
    }
}
pub struct AuthenticatedSession {
    pub channel: SealedChannel,
    pub ready: Ready,
}
pub async fn session<S: AsyncRead + AsyncWrite + Unpin>(
    connection: &mut Connection<S>,
    identity: Identity,
    paired: PairedPeer,
    ready: Control,
) -> io::Result<AuthenticatedSession> {
    let start = Instant::now();
    let result = time::timeout(Duration::from_secs(5), async {
        let (mut machine, hello) = Session::pc(identity, paired, ready, 0)?;
        connection.sender.enqueue(&hello)?;
        loop {
            if machine.is_failed() && connection.sender.is_empty() { return Err(crate::rejected(machine.failure().unwrap_or("bad_message"))); }
            if machine.is_done() && connection.sender.is_empty() {
                let channel=machine.take_channel().ok_or_else(||invalid("Session did not authenticate."))?;
                let ready=machine.take_peer_ready().ok_or_else(||invalid("Peer ready is missing."))?;
                return Ok(AuthenticatedSession { channel,ready });
            }
            tokio::select! {
                frame = connection.receiver.receive(), if !machine.is_done() && !machine.is_failed() => {
                    let outbound = machine.receive(&frame?, start.elapsed().as_millis() as u64)?;
                    for frame in outbound { connection.sender.enqueue(&frame)?; }
                }
                result = connection.sender.flush(), if !connection.sender.is_empty() => { result?; }
            }
        }
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Session handshake timed out."))
    .and_then(|result| result);
    if result.is_err() {
        connection.close().await;
    }
    result
}
