---
order: 4
---

### The `conbind-extension://` protocol

When Conbind renders an extension iframe, it loads a URL like:

```
conbind-extension://com.yourname.hello-world/index.html?view=DefaultView
```

The Rust URI scheme handler (`uri_schemes.rs`) resolves this URL to a file with the following priority order:

1. **Dev extensions** — paths registered in `dev_extensions.json` (via `conbind link` or Create Extension). Enables hot-reload during development.
2. **Debug fallback** (debug builds only) — `src/built-in-features/{id}/dist/`.
3. **Built-in resources** — files bundled into the app binary.
4. **Installed extensions** — `$APP_DATA/extensions/{extensionId}/dist/`.

The protocol handler strips query parameters and URL fragments before filesystem lookup. It also has path-traversal protection (`..` segments are rejected).

**Security:** The iframe runs with this sandbox attribute:

```
allow-scripts allow-same-origin allow-forms allow-popups
```

And this Content Security Policy:

```
default-src conbind-extension: 'self';
script-src  conbind-extension: 'unsafe-inline' 'unsafe-eval';
style-src   conbind-extension: 'unsafe-inline';
font-src    conbind-extension:;
img-src     conbind-extension: data:;
```

External URLs in `<script src="">` tags are blocked. All networking goes through `NetworkService`.

### Host Window Asset Resolution

The `conbind-extension://` protocol is also used by the host application (Settings and launcher search results) to load extension-bundled icon images (e.g. `conbind-extension://<extensionId>/icon.png`). The host window's Content Security Policy allows `conbind-extension:` in `img-src`, and the Rust protocol handler validates that the file is located within an approved extension directory before serving it.
