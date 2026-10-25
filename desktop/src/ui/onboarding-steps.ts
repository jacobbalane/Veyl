// Onboarding's steps (§7.12, Appendix C onboarding 1–7). Both paths share the welcome, the choice
// of how to use Veyl, the practice poke and the actions step.
const phoneSteps = [
  'welcome',
  'choose',
  'phone-app',
  'pair',
  'place',
  'phone-play',
  'practice',
  'actions',
] as const;
const pcSteps = ['welcome', 'choose', 'pc-devices', 'pc-play', 'practice', 'actions'] as const;
export type OnboardingStep = (typeof phoneSteps)[number] | (typeof pcSteps)[number];
/** Every step once, for the preview's `?onboarding=` option and tests. */
export const onboardingSteps: readonly OnboardingStep[] = [
  ...new Set<OnboardingStep>([...phoneSteps, ...pcSteps]),
];
/** The steps in order for one way of using Veyl. */
export function onboardingPath(mode: 'phone' | 'pc'): readonly OnboardingStep[] {
  return mode === 'pc' ? pcSteps : phoneSteps;
}
