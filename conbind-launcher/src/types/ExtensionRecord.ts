import type { ExtensionManifest } from 'conbind-sdk/contracts';
import type { CompatibilityStatus } from './CompatibilityStatus';

export interface ExtensionRecord {
  manifest: ExtensionManifest;
  enabled: boolean;
  isBuiltIn: boolean;
  disableable?: boolean;
  path: string;
  compatibility?: CompatibilityStatus;
  firstViewComponent?: string | null;
}
