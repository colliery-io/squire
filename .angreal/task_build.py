"""Build tasks: the Rust workspace and the signed Android release APK."""

import angreal

from utils import gradle, run


@angreal.command(name="build", about="Build the Rust workspace (debug; pass --release for optimized)")
@angreal.argument(
    name="release", long="release", is_flag=True, takes_value=False,
    help="Optimized release build",
)
def build(release=False):
    cmd = ["cargo", "build"]
    if release:
        cmd.append("--release")
    return run(cmd)


@angreal.command(name="apk", about="Build the signed release APK (com.squire.app)")
def apk():
    rc = gradle(":app:assembleRelease")
    if rc == 0:
        print("APK → clients/squire-android/app/build/outputs/apk/release/app-release.apk")
    return rc
