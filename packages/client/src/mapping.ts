// Core JSON <-> the UI's view models. Pure functions; no secrets are fetched here.
import type { FieldView, ItemDraft, ItemKind, ItemView } from '@scytale/ui/types';
import type { CoreItem, CoreItemData, CoreView } from './engine';

export function emptyDraft(kind: ItemKind = 'login'): ItemDraft {
  return {
    kind,
    title: '',
    notes: '',
    favorite: false,
    username: '',
    password: '',
    urls: '',
    totp: '',
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
}

export function draftFromItem(item: CoreItem): ItemDraft {
  const d = { ...emptyDraft(item.data.type), title: item.title, notes: item.notes, favorite: item.favorite };
  const data = item.data;
  switch (data.type) {
    case 'login':
      return { ...d, username: data.username, password: data.password, urls: data.urls.join('\n'), totp: data.totp, exactHost: data.exact_host };
    case 'card':
      return { ...d, holder: data.holder, number: data.number, expiry: data.expiry, cvv: data.cvv };
    case 'identity':
      return { ...d, fullName: data.full_name, email: data.email, phone: data.phone, address: data.address1 };
    case 'note':
      return d;
  }
}

/** Applies a draft. Fields the form does not show (card PIN, identity city…) keep their old values. */
export function itemFromDraft(d: ItemDraft, original?: CoreItem): CoreItem {
  const old = original?.data.type === d.kind ? original.data : undefined;
  let data: CoreItemData;
  switch (d.kind) {
    case 'login':
      data = {
        type: 'login',
        username: d.username.trim(),
        password: d.password,
        urls: d.urls.split(/\r?\n/).map((u) => u.trim()).filter(Boolean),
        totp: d.totp.trim(),
        exact_host: d.exactHost,
      };
      break;
    case 'card': {
      const prev = old?.type === 'card' ? old : undefined;
      data = { type: 'card', holder: d.holder.trim(), number: d.number.trim(), expiry: d.expiry.trim(), cvv: d.cvv.trim(), pin: prev?.pin ?? '' };
      break;
    }
    case 'identity': {
      const prev = old?.type === 'identity' ? old : undefined;
      data = {
        type: 'identity',
        full_name: d.fullName.trim(),
        email: d.email.trim(),
        phone: d.phone.trim(),
        company: prev?.company ?? '',
        address1: d.address.trim(),
        address2: prev?.address2 ?? '',
        city: prev?.city ?? '',
        region: prev?.region ?? '',
        postal_code: prev?.postal_code ?? '',
        country: prev?.country ?? '',
      };
      break;
    }
    case 'note':
      data = { type: 'note' };
  }
  return { title: d.title.trim(), notes: d.notes, favorite: d.favorite, data };
}

/** Only http(s) links are ever rendered: an imported `javascript:` URL must not become clickable. */
export function safeHref(url: string): string | undefined {
  try {
    const u = new URL(url.includes('://') ? url : `https://${url}`);
    return u.protocol === 'https:' || u.protocol === 'http:' ? u.href : undefined;
  } catch {
    return undefined;
  }
}

const rtf = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });

export function relativeTime(ms: number, now: number): string {
  const secs = Math.round((ms - now) / 1000);
  const abs = Math.abs(secs);
  if (abs < 45) return 'just now';
  if (abs < 3600) return rtf.format(Math.round(secs / 60), 'minute');
  if (abs < 86400) return rtf.format(Math.round(secs / 3600), 'hour');
  if (abs < 86400 * 30) return rtf.format(Math.round(secs / 86400), 'day');
  return `on ${new Date(ms).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' })}`;
}

export function shortDate(ms: number): string {
  return new Date(ms).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' });
}

export function itemView(
  v: CoreView,
  opts: { now: number; deviceName: string; revealed: Record<string, string>; totp?: ItemView['totp'] },
): ItemView {
  const data = v.item.data;
  const has = (s: string) => v.secrets.includes(s);
  const secret = (key: string, label: string, copyable = true): FieldView => ({
    key,
    label,
    secret: true,
    copyable,
    value: opts.revealed[key],
  });
  const plain = (key: string, label: string, value: string, copyable = false): FieldView[] =>
    value ? [{ key, label, value, copyable }] : [];

  let fields: FieldView[] = [];
  switch (data.type) {
    case 'login':
      fields = [
        ...plain('username', 'Username', data.username, true),
        ...(has('password') ? [secret('password', 'Password')] : []),
        ...data.urls.map((u, i) => ({ key: `url-${i}`, label: i === 0 ? 'Website' : '', value: u, href: safeHref(u) })),
      ];
      break;
    case 'card':
      fields = [
        ...plain('holder', 'Name on card', data.holder),
        ...(has('number') ? [secret('number', 'Card number')] : []),
        ...plain('expiry', 'Expiry', data.expiry),
        ...(has('cvv') ? [secret('cvv', 'Security code')] : []),
        ...(has('pin') ? [secret('pin', 'PIN')] : []),
      ];
      break;
    case 'identity':
      fields = [
        ...plain('full_name', 'Full name', data.full_name, true),
        ...plain('email', 'Email', data.email, true),
        ...plain('phone', 'Phone', data.phone, true),
        ...plain('company', 'Company', data.company),
        ...plain(
          'address',
          'Address',
          [data.address1, data.address2, data.city, data.region, data.postal_code, data.country].filter(Boolean).join(', '),
          true,
        ),
      ];
      break;
    case 'note':
      break;
  }

  return {
    id: v.id,
    title: v.item.title,
    kind: data.type,
    edited: `Edited ${relativeTime(v.edited_ms, opts.now)} ${opts.deviceName}`,
    favorite: v.item.favorite,
    fields,
    totp: opts.totp,
    history: v.history.map((ms) => ({ label: '••••••••••••', until: `Until ${shortDate(ms)}` })),
    notes: v.item.notes,
  };
}
