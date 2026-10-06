// Dropbox app key ("client id"): public by design, no secret involved (OAuth PKCE). Set
// WXT_DROPBOX_CLIENT_ID when building; without it the Dropbox option is not offered.
// Register one at https://www.dropbox.com/developers/apps (Scoped access, App folder,
// files.content.read + files.content.write, redirect URI = the extension's identity redirect).
export const DROPBOX_CLIENT_ID: string = import.meta.env.WXT_DROPBOX_CLIENT_ID ?? '';
