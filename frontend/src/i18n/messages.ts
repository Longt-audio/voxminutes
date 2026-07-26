import type { Language } from './languages'
import { CORE_MESSAGES, type CoreMessages } from './areas/core'
import { RECORDER_MESSAGES, type RecorderMessages } from './areas/recorder'
import { HISTORY_MESSAGES, type HistoryMessages } from './areas/history'
import { SETTINGS_MESSAGES, type SettingsMessages } from './areas/settings'
import { TRANSLATE_MESSAGES, type TranslateMessages } from './areas/translate'
import { SUMMARY_MESSAGES, type SummaryMessages } from './areas/summary'
import { ONBOARDING_MESSAGES, type OnboardingMessages } from './areas/onboarding'

export type { Language } from './languages'
export { LANGUAGE_OPTIONS } from './languages'

export type Messages = CoreMessages &
  RecorderMessages &
  HistoryMessages &
  SettingsMessages &
  TranslateMessages &
  SummaryMessages &
  OnboardingMessages

export const MESSAGES: Record<Language, Messages> = {
  en: {
    ...CORE_MESSAGES.en,
    ...RECORDER_MESSAGES.en,
    ...HISTORY_MESSAGES.en,
    ...SETTINGS_MESSAGES.en,
    ...TRANSLATE_MESSAGES.en,
    ...SUMMARY_MESSAGES.en,
    ...ONBOARDING_MESSAGES.en,
  },
  zh: {
    ...CORE_MESSAGES.zh,
    ...RECORDER_MESSAGES.zh,
    ...HISTORY_MESSAGES.zh,
    ...SETTINGS_MESSAGES.zh,
    ...TRANSLATE_MESSAGES.zh,
    ...SUMMARY_MESSAGES.zh,
    ...ONBOARDING_MESSAGES.zh,
  },
  ko: {
    ...CORE_MESSAGES.ko,
    ...RECORDER_MESSAGES.ko,
    ...HISTORY_MESSAGES.ko,
    ...SETTINGS_MESSAGES.ko,
    ...TRANSLATE_MESSAGES.ko,
    ...SUMMARY_MESSAGES.ko,
    ...ONBOARDING_MESSAGES.ko,
  },
  ja: {
    ...CORE_MESSAGES.ja,
    ...RECORDER_MESSAGES.ja,
    ...HISTORY_MESSAGES.ja,
    ...SETTINGS_MESSAGES.ja,
    ...TRANSLATE_MESSAGES.ja,
    ...SUMMARY_MESSAGES.ja,
    ...ONBOARDING_MESSAGES.ja,
  },
}
