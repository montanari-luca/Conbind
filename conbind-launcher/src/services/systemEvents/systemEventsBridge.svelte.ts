import { createPushBridge } from '../eventPushBridge/createPushBridge';

/** Routes Rust subscription events through the shared worker-aware delivery bridge. */
export const systemEventsBridge = createPushBridge(
  'conbind:system-event',
  'conbind:event:system-event:push',
  'systemEventsBridge',
);
