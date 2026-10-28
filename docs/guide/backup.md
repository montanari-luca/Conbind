# Backup

> Export your settings and data to a file, and restore them anywhere.

## Export & import

Backups let you save a snapshot of your Conbind data to a file and restore it on the same computer or a different one. Conbind has no accounts and no cloud sync, so a backup file is how you move your setup between machines.

**Exporting a backup:**

1. Open **Settings → Backup**.
2. Check the data categories you want to include (for example, Snippets, Agents, Shortcuts).
3. If the backup includes sensitive data such as API keys, Conbind warns you. You can set an optional password to encrypt the file; if you leave it blank, sensitive fields are stripped from the exported file automatically.
4. Click **Export…** and choose where to save the file.

**Importing a backup:**

1. Open **Settings → Backup**.
2. Click **Choose Backup File…** and select a previously exported backup.
3. If the file is password-protected, enter the password when prompted.
4. A preview dialog shows each category in the backup along with counts (local vs. incoming) and any conflicts.
5. For each category, choose how to handle conflicts: **Merge** (keep both), **Replace** (overwrite with backup data), or **Skip** (leave your current data untouched).
6. Click **Restore** to apply.

## Related

- [Settings](./settings.md)
- [FAQ](./faq.md)
- [Troubleshooting](./troubleshooting.md)
