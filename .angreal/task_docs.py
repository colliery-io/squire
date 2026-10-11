"""Docs tasks: build/serve the Starlight doc site and refresh its screenshots.

The site lives in ``docs/``: Astro Starlight with the Colliery theme starlight-aurora (SQUIRE-T-0148).
It needs Node 22 or later. It's authored here and published to this repo's GitHub Pages by
``.github/workflows/docs.yml`` — see ``.github/RELEASING.md``. Screenshots are committed under
``docs/src/assets/images/`` so the CI docs build stays fast; ``angreal docs shots`` regenerates them
from the live Keep (Playwright) and the Android Paparazzi goldens.
"""

import shutil

import angreal

from utils import bash, root, run

docs = angreal.command_group(name="docs", about="Build, serve, and screenshot the doc site")

# Keep web UI tabs captured by e2e/tests/gallery.spec.ts → docs image name.
_KEEP_SHOTS = {
    "quests": "keep-quests.png",
    "members": "keep-members.png",
    "pair": "keep-pair.png",
    "settings": "keep-settings.png",
    "log": "keep-log.png",
}

# Android Paparazzi golden (without the test-class prefix) → docs image name.
_PHONE_SHOTS = {
    "squirePlayerHome": "squire-home.png",
    "squirePlayerRewards": "squire-rewards.png",
    "squirePlayerMe": "squire-me.png",
    "knightReviewHome": "knight-review.png",
    "knightManageQuests": "knight-manage-quests.png",
    "knightAddFunds": "knight-add-funds.png",
}

_PAPARAZZI_DIR = "clients/squire-android/app/src/test/snapshots/images"
_PAPARAZZI_PREFIX = "com.squire.app.screenshots_ScreenshotTests"


def _npm(*args):
    """Run npm in docs/, installing the dependencies on the first run."""
    npm = shutil.which("npm")
    if not npm:
        print("error: npm not found: install Node 22 or later")
        return 1
    d = root() / "docs"
    if not (d / "node_modules").exists():
        rc = run([npm, "ci", "--no-audit", "--no-fund"], cwd=d)
        if rc != 0:
            return rc
    return run([npm, *args], cwd=d)


@docs()
@angreal.command(
    name="build",
    about="Build the Starlight doc site into docs/dist",
    tool=angreal.ToolDescription(
        "Builds the Starlight docs site in docs/ into docs/dist. Installs the npm dependencies on "
        "the first run. Fails when a page does not build.",
        risk_level="safe",
    ),
)
def docs_build():
    return _npm("run", "build")


@docs()
@angreal.command(
    name="serve",
    about="Serve the doc site locally with live reload (http://localhost:4321)",
    tool=angreal.ToolDescription(
        "Serves the docs site at http://localhost:4321 and reloads on each change. In a terminal it "
        "runs until Ctrl+C. From a script or an agent, Astro runs it in the background: stop it "
        "with `npx astro dev stop` in docs/.",
        risk_level="safe",
    ),
)
def docs_serve():
    return _npm("run", "dev")


@docs()
@angreal.command(
    name="shots",
    about="Refresh docs/src/assets/images: re-run the Keep Playwright gallery + copy the Paparazzi goldens",
)
@angreal.argument(
    name="record", long="record", is_flag=True, takes_value=False,
    help="Re-record the Android Paparazzi goldens first (slow; needs the Android toolchain)",
)
def docs_shots(record=False):
    r = root()
    images = r / "docs" / "src" / "assets" / "images"
    images.mkdir(parents=True, exist_ok=True)

    # 1. Keep web UI — Playwright boots its own throwaway demo server, then writes e2e/screens/.
    rc = bash("npx playwright test tests/gallery.spec.ts", cwd=r / "e2e")
    if rc != 0:
        print("error: Keep gallery capture failed (did you run `npm install` in e2e/?)")
        return rc
    for tab, name in _KEEP_SHOTS.items():
        shutil.copyfile(r / "e2e" / "screens" / f"tab-{tab}.png", images / name)

    # 2. Phone app — Compose can't be Playwright-driven, so we reuse the Paparazzi goldens.
    if record:
        rc = run(["angreal", "test", "record"])
        if rc != 0:
            return rc
    pap = r / _PAPARAZZI_DIR
    for golden, name in _PHONE_SHOTS.items():
        shutil.copyfile(pap / f"{_PAPARAZZI_PREFIX}_{golden}.png", images / name)

    print(f"Refreshed {len(_KEEP_SHOTS) + len(_PHONE_SHOTS)} screenshots → docs/src/assets/images/")
    return 0
