# Keyring Status Report — User: sandboxed (UID 1001)

**Date:** 2026-03-09
**Host OS:** Fedora (Linux 6.18.6-200.fc43.x86_64)

---

## 1. Executive Summary

The GNOME Keyring for user `sandboxed` is non-functional. The login collection is advertised by the Secret Service D-Bus interface but does not actually exist as a D-Bus object. No login path on this system is configured to unlock the keyring for this user. Applications relying on the default keyring for credential storage will fail silently or error out.

---

## 2. Environment

### 2.1 Active Sessions

There are 11 sessions across 4 users on the system. The `sandboxed` user has 4 sessions:

| Session | PID    | Class   | TTY   | Idle         |
|---------|--------|---------|-------|--------------|
| 4       | 6042   | manager | -     | no           |
| c89     | 1497516| user    | pts/0 | 2 days       |
| c97     | 821781 | user    | pts/4 | no           |
| c108    | 1873849| user    | pts/1 | no           |

Other logged-in users: `ahmed` (UID 1000), `sandbox-test` (UID 1002), `botminter-test` (UID 1003).

### 2.2 gnome-keyring-daemon Processes

| PID     | UID  | User         | Args                                              | Since  |
|---------|------|--------------|----------------------------------------------------|--------|
| 334285  | 1000 | ahmed        | `--start --foreground --components=secrets`         | Feb 23 |
| 2873624 | 1001 | sandboxed    | `--start --foreground --components=secrets`         | Mar 02 |
| 2848324 | 1001 | sandboxed    | `--unlock --components=secret`                     | Mar 09 |
| 1405836 | 1002 | sandbox-test | `--start --foreground --components=secrets`         | Mar 06 |

The `sandboxed` user has two daemon instances:
- **PID 2873624** — the main daemon, running since Mar 02, started with `--start`.
- **PID 2848324** — a manual `--unlock` attempt made today (Mar 09), running on `pts/23`. This instance is isolated and not connected to the main daemon's D-Bus session, so it had no effect.

---

## 3. Kernel Keyring Status

| Keyring              | Key ID     | Status                          |
|----------------------|------------|---------------------------------|
| `_uid.1001`          | 0x1781b957 | Exists, **empty** (no keys)     |
| `_uid_ses.1001`      | 0x33f04411 | Exists, contains `_uid.1001`    |
| Session keyring `@s` | -          | **Revoked** — inaccessible      |

Multiple session keyrings exist in `/proc/keys` for UID 1001. One (0x160ec1b7) is **expired**. This is a consequence of multiple sessions being active with some having gone stale.

---

## 4. GNOME Keyring / Secret Service Status

### 4.1 Collections Reported

The D-Bus Secret Service interface (`org.freedesktop.secrets`) reports two collections:

| Collection Path                                  | Exists | Locked | Label |
|--------------------------------------------------|--------|--------|-------|
| `/org/freedesktop/secrets/collection/session`    | Yes    | No     | ""    |
| `/org/freedesktop/secrets/collection/login`      | **No** | -      | -     |

### 4.2 The Login Collection Problem

- The `Collections` property lists `/org/freedesktop/secrets/collection/login`.
- The `default` alias resolves to `/org/freedesktop/secrets/collection/login`.
- **However**, querying any property (`Label`, `Locked`, `Items`) on that path returns: `Object does not exist at path "/org/freedesktop/secrets/collection/login"`.

This means the login collection is a **phantom entry** — referenced but never materialized. The `session` collection (transient, wiped on logout) is the only functional collection.

### 4.3 Impact

- Any application using `libsecret`, `secret-tool`, or the D-Bus Secret Service API to store/retrieve credentials against the default collection will fail.
- Affected applications may include: Git credential helpers, GNOME Online Accounts, SSH key agents, application tokens, etc.

---

## 5. Root Cause Analysis

### 5.1 PAM Configuration

The keyring is unlocked at login time via PAM. The relevant PAM files on this system:

| PAM File               | `pam_gnome_keyring.so` Present | Status       |
|------------------------|-------------------------------|--------------|
| `/etc/pam.d/login`     | Yes                           | **Commented out** |
| `/etc/pam.d/lightdm`   | Yes                           | Active       |
| `/etc/pam.d/passwd`    | Yes                           | Active (password sync only) |
| `/etc/pam.d/su-l`      | **No**                        | Missing      |
| `/etc/pam.d/su`        | **No**                        | Missing      |

### 5.2 Login Path

The `sandboxed` user is accessed via `su -` (which uses `/etc/pam.d/su-l`). This PAM configuration does not include `pam_gnome_keyring.so`, so:

1. The keyring daemon starts (via systemd user service or XDG autostart) but is **never given the user's password**.
2. Without the password, the daemon cannot decrypt the login collection's backing store (`~/.local/share/keyrings/login.keyring`).
3. The login collection is registered in the daemon's internal state but never instantiated as a D-Bus object.

### 5.3 Failed Manual Unlock

The `--unlock` attempt (PID 2848324) failed because:
- It started a **new** daemon instance rather than communicating with the existing one (PID 2873624).
- It was not run with `--replace`, so it could not take over the D-Bus name.
- It was launched on a different TTY (`pts/23`), likely in a different D-Bus session context.

---

## 6. Recommended Fix

### Option A: Add keyring unlock to `su -` (Recommended)

Edit `/etc/pam.d/su-l` as root to add two lines:

```
#%PAM-1.0
auth        include     su
auth        optional    pam_gnome_keyring.so
account     include     su
password    include     su
session     optional    pam_keyinit.so force revoke
session     include     su
session     optional    pam_gnome_keyring.so auto_start
```

- `auth optional pam_gnome_keyring.so` — captures the password during authentication.
- `session optional pam_gnome_keyring.so auto_start` — starts and unlocks the keyring daemon with the captured password.
- Both are `optional`, so `su -` functionality is unaffected if the keyring module fails.

After making this change, the next `su - sandboxed` will automatically unlock the login collection.

### Option B: Immediate manual unlock

To unlock right now without modifying PAM:

```bash
echo -n "<password>" | gnome-keyring-daemon --replace --unlock
```

This replaces the existing daemon and unlocks the login collection. The effect is temporary — it must be repeated after each login.

### Option C: Uncomment in `/etc/pam.d/login`

If the user ever logs in via console (`login`), uncomment the two `pam_gnome_keyring.so` lines in `/etc/pam.d/login`. This does not help the `su -` case.

---

## 7. Cleanup Recommendation

Kill the orphaned unlock daemon that had no effect:

```bash
kill 2848324
```

This process is not serving any purpose and is consuming a small amount of resources.

---

## 8. Verification Steps

After applying the fix, verify with:

```bash
# Check login collection exists and is unlocked
busctl --user get-property org.freedesktop.secrets \
  /org/freedesktop/secrets/collection/login \
  org.freedesktop.Secret.Collection Locked
# Expected: b false

# Check items can be listed
busctl --user get-property org.freedesktop.secrets \
  /org/freedesktop/secrets/collection/login \
  org.freedesktop.Secret.Collection Items
# Expected: ao <count> ...

# Test storing a secret
secret-tool store --label="test" service test-keyring username test
secret-tool lookup service test-keyring
secret-tool clear service test-keyring
```
