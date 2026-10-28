// conbind-launcher/src/lib/ipc/oauthCommands.ts
// Tauri command wrappers, re-exported through ./commands (the barrel).
import { invokeSafe } from './invokeSafe';

// ── OAuth PKCE for Extensions ─────────────────────────────────────────────────

export interface OAuthStartResponse {
  state: string;
  authUrl: string;
}

export interface OAuthTokenPayload {
  accessToken: string;
  refreshToken?: string;
  tokenType: string;
  scopes: string[];
  /** Unix timestamp seconds. Undefined = no expiry. */
  expiresAt?: number;
}

export interface OAuthExchangeResponse {
  extensionId: string;
  flowId: string;
  token: OAuthTokenPayload;
}

export async function oauthStartFlow(
  extensionId: string,
  providerId: string,
  clientId: string,
  authorizationUrl: string,
  tokenUrl: string,
  scopes: string[],
  flowId: string,
): Promise<OAuthStartResponse> {
  return invokeSafe<OAuthStartResponse>('oauth_start_flow', {
    extensionId,
    providerId,
    clientId,
    authorizationUrl,
    tokenUrl,
    scopes,
    flowId,
  });
}

export async function oauthExchangeCode(
  stateParam: string,
  code: string,
): Promise<OAuthExchangeResponse> {
  return invokeSafe<OAuthExchangeResponse>('oauth_exchange_code', { stateParam, code });
}

export async function oauthGetStoredToken(
  extensionId: string,
  providerId: string,
): Promise<OAuthTokenPayload | null> {
  return invokeSafe<OAuthTokenPayload | null>('oauth_get_stored_token', {
    extensionId,
    providerId,
  });
}

export async function oauthRevokeExtensionToken(
  extensionId: string,
  providerId: string,
): Promise<void> {
  await invokeSafe('oauth_revoke_extension_token', { extensionId, providerId });
}
