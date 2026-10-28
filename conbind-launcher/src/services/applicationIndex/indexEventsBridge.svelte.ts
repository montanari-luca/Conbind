import { createPushBridge } from '../eventPushBridge/createPushBridge';

/** Routes Rust subscription events through the shared worker-aware delivery bridge. */
export const indexEventsBridge = createPushBridge(
  'conbind:application-index',
  'conbind:event:application-index:push',
  'indexEventsBridge',
);
