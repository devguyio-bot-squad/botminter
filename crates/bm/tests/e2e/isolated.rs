//! Isolated E2E tests — smoke checks and standalone tests that don't fit in scenarios.

use libtest_mimic::Trial;

use super::helpers::{run_test, E2eConfig};
use super::test_env::TestEnv;

pub fn tests(config: &E2eConfig) -> Vec<Trial> {
    let cfg = config.clone();
    vec![
        Trial::test("e2e_harness_smoke", {
            let cfg = cfg.clone();
            move || {
                run_test(|| {
                    // GitHub smoke
                    let repo =
                        super::github::TempRepo::new_in_org("bm-e2e-smoke", &cfg.gh_org)
                            .expect("GitHub repo creation failed");
                    let labels = super::github::list_labels(&repo.full_name);
                    eprintln!(
                        "GitHub smoke: repo {} has {} default labels",
                        repo.full_name,
                        labels.len()
                    );

                    let issues = super::github::list_issues(&repo.full_name);
                    assert!(
                        issues.is_empty(),
                        "fresh repo should have no issues, found: {:?}",
                        issues
                    );

                    // Telegram smoke
                    if super::telegram::podman_available() {
                        let mock = super::telegram::TgMock::start();
                        let token = "test-token-smoke";
                        let chat_id = 12345i64;
                        mock.inject_message(token, "hello from e2e smoke test", chat_id);
                        let requests = mock.get_requests(token, "sendMessage");
                        eprintln!(
                            "Telegram smoke: tg-mock has {} sendMessage requests",
                            requests.len()
                        );
                    } else {
                        eprintln!("SKIP: podman not available -- skipping tg-mock smoke");
                    }
                })
            }
        }),
        Trial::test("e2e_list_gh_projects", {
            let cfg = cfg.clone();
            move || {
                run_test(|| {
                    let env = TestEnv::fresh(&cfg.gh_token, &cfg.gh_org, "");

                    let project = super::github::TempProject::new(
                        &env,
                        &cfg.gh_org,
                        "bm-e2e-list-projects",
                    )
                    .expect("Failed to create temp GitHub Project");

                    let projects =
                        bm::git::list_projects(&cfg.gh_token, &cfg.gh_org)
                            .expect("list_gh_projects should succeed");

                    let found = projects.iter().find(|(n, _)| *n == project.number);
                    assert!(
                        found.is_some(),
                        "list_gh_projects should include project #{}, got: {:?}",
                        project.number,
                        projects
                    );

                    let (_, title) = found.unwrap();
                    assert_eq!(title, "bm-e2e-list-projects");

                    // Idempotency
                    let projects2 =
                        bm::git::list_projects(&cfg.gh_token, &cfg.gh_org)
                            .expect("second list_gh_projects should succeed");
                    let found2 = projects2.iter().find(|(n, _)| *n == project.number);
                    assert!(found2.is_some());
                })
            }
        }),
        Trial::test("e2e_keyring_isolated", {
            move || {
                run_test(|| {
                    use bm::bridge::CredentialStore;

                    // TestEnv creates isolated D-Bus + gnome-keyring-daemon
                    let env = TestEnv::fresh(&cfg.gh_token, &cfg.gh_org, "");

                    let state_path = env.home().join("bridge-state.json");

                    // LocalCredentialStore reads BM_KEYRING_DBUS from the process env.
                    // Since this test calls Rust API in-process (not via subprocess),
                    // we must set it for the current process. Known deviation from
                    // ADR-005's no-process-wide-env rule, necessary because the store
                    // API is called in-process rather than via CLI subprocess.
                    let resolved = env.resolved_env("bm");
                    let dbus_addr = resolved.get("BM_KEYRING_DBUS")
                        .expect("BM_KEYRING_DBUS should be in resolved env for bm");

                    // Safety: tests run single-threaded (--test-threads=1).
                    // RAII guard ensures cleanup even if assertions panic
                    // (run_test uses catch_unwind).
                    struct EnvGuard;
                    impl Drop for EnvGuard {
                        fn drop(&mut self) {
                            unsafe { std::env::remove_var("BM_KEYRING_DBUS"); }
                        }
                    }
                    unsafe { std::env::set_var("BM_KEYRING_DBUS", dbus_addr); }
                    let _env_guard = EnvGuard;

                    let store = bm::bridge::LocalCredentialStore::new(
                        "e2e-keyring-test", "telegram", state_path,
                    );

                    // Store
                    store.store("test-member", "secret-token-123").expect("store failed");

                    // Retrieve
                    let token = store.retrieve("test-member").expect("retrieve failed");
                    assert_eq!(token.as_deref(), Some("secret-token-123"), "token mismatch");

                    // Reset keyring (simulates reset_home)
                    env.reset_keyring();

                    // After reset, credential should be gone
                    let after = store.retrieve("test-member").expect("retrieve after reset failed");
                    assert_eq!(after, None, "credential should be gone after reset");

                    // Store again after reset (proves re-setup works)
                    store.store("test-member", "new-token-456").expect("store after reset failed");
                    let token2 = store.retrieve("test-member").expect("retrieve2 failed");
                    assert_eq!(token2.as_deref(), Some("new-token-456"), "token2 mismatch");

                    eprintln!("Keyring isolation: store/retrieve/reset/re-store all passed");
                })
            }
        }),
    ]
}
