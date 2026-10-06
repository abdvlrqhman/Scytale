// Presentational only: props in, callbacks out. No imports from the core or the client layer.
export { default as Button } from './lib/Button.svelte';
export { default as Icon, type IconName } from './lib/Icon.svelte';
export { default as IconButton } from './lib/IconButton.svelte';
export { default as ItemRow } from './lib/ItemRow.svelte';
export { default as Monogram } from './lib/Monogram.svelte';
export { default as PasswordField } from './lib/PasswordField.svelte';
export { default as ScytaleStrip } from './lib/ScytaleStrip.svelte';
export { default as SearchField } from './lib/SearchField.svelte';
export { default as SegmentedControl } from './lib/SegmentedControl.svelte';
export { default as StrengthMeter } from './lib/StrengthMeter.svelte';
export { default as Switch } from './lib/Switch.svelte';
export { default as SyncBadge } from './lib/SyncBadge.svelte';
export { default as TextField } from './lib/TextField.svelte';
export { default as TitleBar } from './lib/TitleBar.svelte';
export { default as Toast } from './lib/Toast.svelte';
export { default as TotpCode } from './lib/TotpCode.svelte';
export { default as Wordmark } from './lib/Wordmark.svelte';

export { default as EmergencyKit } from './screens/EmergencyKit.svelte';
export { default as Generator } from './screens/Generator.svelte';
export { default as ItemDetail } from './screens/ItemDetail.svelte';
export { default as ItemEdit } from './screens/ItemEdit.svelte';
export { default as Onboarding } from './screens/Onboarding.svelte';
export { default as Popup } from './screens/Popup.svelte';
export { default as Settings } from './screens/Settings.svelte';
export { default as Unlock } from './screens/Unlock.svelte';
export { default as VaultWindow, type Section } from './screens/VaultWindow.svelte';

export type {
  DeviceView,
  FieldView,
  GeneratorOptions,
  ItemDraft,
  ItemKind,
  ItemSummary,
  ItemView,
  StorageProvider,
  Strength,
  SyncInfo,
  SyncStatus,
  Theme,
} from './types';
