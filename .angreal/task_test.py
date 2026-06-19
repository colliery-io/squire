"""Test tasks: Rust workspace, Keep Playwright E2E, and Android Paparazzi snapshots."""

import angreal

from utils import bash, gradle, root, run

test = angreal.command_group(name="test", about="Run the test suites")


def _rust():
    return run(["cargo", "test"])


def _e2e():
    # Playwright boots + tears down its own throwaway demo server (ports 19920/19080).
    return bash("npm test", cwd=root() / "e2e")


def _snapshots():
    return gradle(":app:verifyPaparazziDebug")


@test()
@angreal.command(name="rust", about="Run the Rust workspace tests")
def test_rust():
    return _rust()


@test()
@angreal.command(name="e2e", about="Run the Keep Playwright E2E suite (self-hosting demo)")
def test_e2e():
    return _e2e()


@test()
@angreal.command(name="snapshots", about="Verify Android Paparazzi snapshots against the goldens")
def test_snapshots():
    return _snapshots()


@test()
@angreal.command(name="record", about="Re-record the Android Paparazzi snapshot goldens")
def test_record():
    return gradle(":app:recordPaparazziDebug")


@test()
@angreal.command(name="all", about="Run Rust + E2E + Paparazzi (stops at the first failure)")
def test_all():
    for step in (_rust, _e2e, _snapshots):
        rc = step()
        if rc != 0:
            return rc
    return 0
