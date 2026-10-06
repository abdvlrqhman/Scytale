import type { Theme } from '@scytale/ui/types';
import type { Strength } from '@scytale/ui/types';
import { send } from './messages';

export function applyTheme(theme: Theme) {
  document.documentElement.dataset.theme = theme;
}

/** Copies a value. Secrets also ask the background to clear the clipboard after the set delay. */
export async function copyText(value: string, secret: boolean) {
  await navigator.clipboard.writeText(value);
  if (secret) await send({ type: 'copied' });
}

// zxcvbn's dictionaries are large, so they load only on pages that rate passwords.
const LABELS = ['Too easy to guess', 'Weak', 'Fair', 'Strong', 'Very strong'] as const;
let rater: ((pw: string) => Strength) | null = null;
let loading: Promise<void> | null = null;

export function loadStrength(): Promise<void> {
  loading ??= Promise.all([
    import('@zxcvbn-ts/core'),
    import('@zxcvbn-ts/language-common'),
    import('@zxcvbn-ts/language-en'),
  ]).then(([core, common, en]) => {
    const z = new core.ZxcvbnFactory({
      dictionary: { ...common.dictionary, ...en.dictionary },
      graphs: common.adjacencyGraphs,
      translations: en.translations,
    });
    rater = (pw) => {
      const r = z.check(pw);
      const hint = r.feedback.warning || r.feedback.suggestions[0] || undefined;
      return { score: r.score, label: LABELS[r.score], hint: hint ?? undefined };
    };
  });
  return loading;
}

/** Synchronous rating for the UI; a length-only estimate until the dictionaries arrive. */
export function rate(pw: string): Strength {
  if (rater) return rater(pw);
  const score = Math.min(4, Math.floor(pw.length / 6)) as Strength['score'];
  return { score, label: LABELS[score] };
}

export function download(filename: string, data: BlobPart, type: string) {
  const url = URL.createObjectURL(new Blob([data], { type }));
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

export const today = () => new Date().toISOString().slice(0, 10);
