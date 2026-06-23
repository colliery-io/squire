"""Docs tasks: build/serve the mdBook doc site and refresh its screenshots.

The site lives in ``docs/`` (mdBook). It's authored here and published to the PUBLIC dist repo
(``colliery-io/squire``) GitHub Pages by ``.github/workflows/docs.yml`` — see SQUIRE-T-0119 and
``.github/RELEASING.md``. Screenshots are committed under ``docs/src/images/`` so the CI docs build
stays fast; ``angreal docs shots`` regenerates them from the live Keep (Playwright) and the Android
Paparazzi goldens.
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


@docs()
@angreal.command(name="build", about="Render the mdBook doc site to docs/book")
def docs_build():
    return run(["mdbook", "build", "docs"])


@docs()
@angreal.command(name="serve", about="Serve the doc site locally with live reload (http://localhost:3000)")
def docs_serve():
    return run(["mdbook", "serve", "docs", "--open"])


@docs()
@angreal.command(
    name="shots",
    about="Refresh docs/src/images: re-run the Keep Playwright gallery + copy the Paparazzi goldens",
)
@angreal.argument(
    name="record", long="record", is_flag=True, takes_value=False,
    help="Re-record the Android Paparazzi goldens first (slow; needs the Android toolchain)",
)
def docs_shots(record=False):
    r = root()
    images = r / "docs" / "src" / "images"
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

    print(f"Refreshed {len(_KEEP_SHOTS) + len(_PHONE_SHOTS)} screenshots → docs/src/images/")
    return 0
