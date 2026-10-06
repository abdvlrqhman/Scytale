export type FillResult = 'filled' | 'origin-changed' | 'no-fields';

/**
 * Injected into the page's top frame only (never iframes). Must be self-contained: it is
 * serialized and runs in the page.
 *
 * Refuses if the page navigated since the match was checked (`expectedOrigin`), and only fills
 * fields a person can actually see, so hidden credential-harvesting inputs stay empty.
 */
export function fillPage(expectedOrigin: string, username: string, password: string): FillResult {
  if (location.origin !== expectedOrigin) return 'origin-changed';

  const visible = (el: HTMLElement) => {
    const r = el.getBoundingClientRect();
    if (r.width < 2 || r.height < 2) return false;
    for (let n: Element | null = el; n; n = n.parentElement) {
      const s = getComputedStyle(n);
      if (s.display === 'none' || s.visibility === 'hidden' || Number(s.opacity) < 0.1) return false;
    }
    return true;
  };
  const set = (el: HTMLInputElement, value: string) => {
    el.focus();
    // Go through the native setter so frameworks (React, Vue…) notice the change.
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(el, value);
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
  };

  const inputs = [...document.querySelectorAll('input')].filter((i) => !i.disabled && !i.readOnly && visible(i));
  const isUserField = (i: HTMLInputElement) =>
    ['text', 'email', 'tel', ''].includes(i.type) &&
    /user|login|email|mail|account|name|identifier/i.test(`${i.name} ${i.id} ${i.autocomplete} ${i.placeholder}`);

  const pw = inputs.find((i) => i.type === 'password');
  if (!pw) {
    // Username-first sign-in pages ("Next", then the password on a second step).
    const user = inputs.find(isUserField);
    if (!user || !username) return 'no-fields';
    set(user, username);
    return 'filled';
  }
  const before = inputs.slice(0, inputs.indexOf(pw)).reverse();
  const user = before.find(isUserField) ?? before.find((i) => ['text', 'email', 'tel'].includes(i.type));
  if (user && username) set(user, username);
  set(pw, password);
  return 'filled';
}
