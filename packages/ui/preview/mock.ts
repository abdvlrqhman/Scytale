import type { DeviceView, GeneratorOptions, ItemDraft, ItemSummary, ItemView, Strength, SyncInfo } from '../src/types';

// Fictional sample data for the preview only. The real app gets all of this from the Rust core.
export const items: ItemSummary[] = [
  { id: 'gh', title: 'GitHub', subtitle: 'sam-rivera', kind: 'login', favorite: true },
  { id: 'gh-work', title: 'GitHub (work)', subtitle: 's.rivera@northwind.dev', kind: 'login' },
  { id: 'gmail', title: 'Gmail', subtitle: 'sam.rivera@gmail.com', kind: 'login', favorite: true },
  { id: 'monzo', title: 'Monzo', subtitle: 'sam.rivera@gmail.com', kind: 'login' },
  { id: 'netflix', title: 'Netflix', subtitle: 'family@riveras.net', kind: 'login' },
  { id: 'notion', title: 'Notion', subtitle: 'sam@northwind.dev', kind: 'login' },
  { id: 'visa', title: 'Visa', subtitle: 'Ending in 4242', kind: 'card' },
  { id: 'wifi', title: 'Home Wi-Fi', subtitle: 'Secure note', kind: 'note' },
  { id: 'passport', title: 'Passport', subtitle: 'Sam Rivera', kind: 'identity' },
];

export const matches = items.filter((i) => i.id.startsWith('gh'));

export const sync: SyncInfo = { status: 'ok', text: 'Synced with Dropbox 2 minutes ago' };

export const github: ItemView = {
  id: 'gh',
  title: 'GitHub',
  kind: 'login',
  edited: 'Edited 3 days ago on Firefox on Linux',
  favorite: true,
  strength: 'strong',
  fields: [
    { key: 'username', label: 'Username', value: 'sam-rivera', copyable: true },
    { key: 'password', label: 'Password', secret: true, copyable: true },
    { key: 'website', label: 'Website', value: 'github.com', href: 'https://github.com' },
  ],
  totp: { code: '482913', remaining: 18, period: 30 },
  history: [
    { label: '••••••••••••', until: 'Until 14 Mar 2026' },
    { label: '••••••••••••', until: 'Until 2 Nov 2025' },
  ],
  notes: 'Recovery codes are in the “GitHub recovery codes” note. Personal account, not the work org.',
};

export const githubPassword = 'x7#Qm2-vLp9!Tr4@Kc8wZe';

export const githubDraft: ItemDraft = {
  kind: 'login',
  title: 'GitHub',
  notes: github.notes,
  favorite: true,
  username: 'sam-rivera',
  password: githubPassword,
  urls: 'https://github.com',
  totp: 'otpauth://totp/GitHub:sam-rivera?secret=JBSWY3DPEHPK3PXP',
  exactHost: false,
  holder: '',
  number: '',
  expiry: '',
  cvv: '',
  fullName: '',
  email: '',
  phone: '',
  address: '',
};

export const devices: DeviceView[] = [
  { id: 'd1', name: 'Firefox on Linux', lastSeen: 'Active now', current: true },
  { id: 'd2', name: 'Scytale for Windows', lastSeen: 'Last synced 2 hours ago', current: false },
];

/** Stand-in for zxcvbn, good enough to show the meter. */
export function rate(pw: string): Strength {
  const classes = [/[a-z]/, /[A-Z]/, /\d/, /[^\w]/].filter((r) => r.test(pw)).length;
  const score = Math.min(4, Math.floor(pw.length / 5) + (classes > 2 ? 1 : 0)) as Strength['score'];
  const labels = ['Too easy to guess', 'Weak', 'Fair', 'Strong', 'Very strong'];
  return {
    score,
    label: labels[score] ?? 'Strong',
    hint: score < 3 ? 'Add more words; length matters more than symbols.' : undefined,
  };
}

// A small sample; the real generator uses the EFF list in the Rust core.
const words = 'harbor velvet lantern orbit meadow copper thistle quarry falcon ember walnut glacier pebble saffron tundra marble cobalt juniper canyon ribbon lagoon cinder maple sparrow tidal hollow summit ferret violet bramble'.split(' ');
const pick = <T,>(xs: T[]) => xs[crypto.getRandomValues(new Uint32Array(1))[0]! % xs.length]!;

export function suggestPassphrase(): string {
  return Array.from({ length: 4 }, () => pick(words)).join('-');
}

export function generate(o: GeneratorOptions): { value: string; bits: number } {
  if (o.mode === 'passphrase') {
    const w = Array.from({ length: o.words }, () => pick(words)).map((x) => (o.capitalize ? x[0]!.toUpperCase() + x.slice(1) : x));
    if (o.includeNumber) w[w.length - 1] += String(pick([...'0123456789']));
    // Real generator uses the 7,776-word EFF list (12.9 bits per word).
    return { value: w.join(o.separator), bits: o.words * 12.9 + (o.includeNumber ? 3.3 : 0) };
  }
  let set = 'abcdefghijkmnpqrstuvwxyz';
  if (o.upper) set += 'ABCDEFGHJKLMNPQRSTUVWXYZ';
  if (o.digits) set += '23456789';
  if (o.symbols) set += '!@#$%^&*()-_=+';
  if (!o.avoidAmbiguous) set += 'lIO01o'.slice(0, o.upper && o.digits ? 6 : 1);
  return {
    value: Array.from({ length: o.length }, () => pick([...set])).join(''),
    bits: o.length * Math.log2(set.length),
  };
}
