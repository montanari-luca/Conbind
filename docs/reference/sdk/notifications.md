# Background feedback

OS notifications are a private presentation child of the unified feedback service. Extensions do not receive a notification service and cannot address the native presenter directly.

**Permission required:** `notifications:send`

```ts
import type { IFeedbackService } from 'conbind-sdk/contracts';

const feedback = context.getService<IFeedbackService>('feedback');

const id = await feedback.sendBackground({
  title: 'Sync complete',
  body: 'Your workspace is up to date.',
  actions: [
    {
      id: 'open',
      title: 'Open',
      commandId: 'open-sync-result',
    },
  ],
});

await feedback.dismissBackground(id);
```

Use background feedback only when no Conbind window needs to remain visible while the work runs. Work visible in the launcher belongs in the Feedback Bar; settings work belongs to the settings presenter.

## IPC contract

| Facade method                   | Wire command                             | Permission           |
| ------------------------------- | ---------------------------------------- | -------------------- |
| `sendBackground(options)`       | `conbind:api:feedback:sendBackground`    | `notifications:send` |
| `dismissBackground(feedbackId)` | `conbind:api:feedback:dismissBackground` | `notifications:send` |

The launcher broker injects the extension identity. Rust validates the permission again before native delivery.
