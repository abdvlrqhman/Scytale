/** "Brave on Windows", shown in the devices list of other devices. */
export function deviceName(): string {
  const nav = navigator as Navigator & { userAgentData?: { brands: { brand: string }[]; platform: string } };
  const ua = navigator.userAgent;
  const brands = nav.userAgentData?.brands.map((b) => b.brand) ?? [];
  const known = ['Brave', 'Microsoft Edge', 'Opera', 'Vivaldi', 'Google Chrome'];
  const fromBrands = known.find((k) => brands.includes(k))?.replace('Microsoft ', '').replace('Google ', '');
  const name =
    fromBrands ??
    (/Firefox\//.test(ua) ? 'Firefox' : /Edg\//.test(ua) ? 'Edge' : /OPR\//.test(ua) ? 'Opera' : 'Chromium');
  const platform = nav.userAgentData?.platform || ua;
  const os = /Win/i.test(platform)
    ? 'Windows'
    : /Mac/i.test(platform)
      ? 'macOS'
      : /CrOS|Chrome OS/i.test(platform)
        ? 'ChromeOS'
        : /Linux/i.test(platform)
          ? 'Linux'
          : 'this computer';
  return `${name} on ${os}`;
}
