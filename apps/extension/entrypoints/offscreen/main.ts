// Chrome only: owns the "clear the clipboard after N seconds" timer, because the service worker
// may be asleep when it fires. Writes an empty string through a copy event, then closes itself.
let timer: ReturnType<typeof setTimeout> | undefined;

browser.runtime.onMessage.addListener((msg: unknown, sender) => {
  if (sender.id !== browser.runtime.id) return undefined;
  const m = msg as { target?: string; clearAfterMs?: number };
  if (m.target !== 'offscreen' || typeof m.clearAfterMs !== 'number') return undefined;
  clearTimeout(timer);
  timer = setTimeout(clear, m.clearAfterMs);
  return undefined;
});

function clear() {
  const onCopy = (e: ClipboardEvent) => {
    e.clipboardData?.setData('text/plain', '');
    e.preventDefault();
  };
  document.addEventListener('copy', onCopy);
  document.execCommand('copy');
  document.removeEventListener('copy', onCopy);
  window.close();
}
