const PBKDF2_ITERATIONS = 100_000;
const PASSWORD_ALPHABET = 'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789';

function hexOf(buf: ArrayBuffer | Uint8Array): string {
  const view = buf instanceof Uint8Array ? buf : new Uint8Array(buf);
  return Array.from(view, (b) => b.toString(16).padStart(2, '0')).join('');
}

export async function derivePbkdf2Spec(password: string): Promise<string> {
  if (!globalThis.crypto?.subtle) {
    throw new Error('WebCrypto tidak tersedia di WebView ini.');
  }
  const salt = new Uint8Array(16);
  globalThis.crypto.getRandomValues(salt);
  const keyMat = await globalThis.crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(password),
    'PBKDF2',
    false,
    ['deriveBits']
  );
  const bits = await globalThis.crypto.subtle.deriveBits(
    { name: 'PBKDF2', hash: 'SHA-256', salt, iterations: PBKDF2_ITERATIONS },
    keyMat,
    256
  );
  return `pbkdf2-sha256$${PBKDF2_ITERATIONS}$${hexOf(salt)}$${hexOf(bits)}`;
}

export function generateStrongPassword(length = 16): string {
  const buf = new Uint8Array(length);
  globalThis.crypto.getRandomValues(buf);
  let out = '';
  for (let i = 0; i < length; i++) {
    out += PASSWORD_ALPHABET[buf[i] % PASSWORD_ALPHABET.length];
  }
  return out;
}
