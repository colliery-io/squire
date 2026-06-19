"""Quality tasks: formatting and clippy lints."""

import angreal

from utils import run


@angreal.command(name="fmt", about="Format Rust code (cargo fmt; --check to verify only)")
@angreal.argument(
    name="check", long="check", is_flag=True, takes_value=False,
    help="Verify formatting without modifying files",
)
def fmt(check=False):
    cmd = ["cargo", "fmt"]
    if check:
        cmd += ["--", "--check"]
    return run(cmd)


@angreal.command(name="clippy", about="Run clippy lints across the workspace")
def clippy():
    return run(["cargo", "clippy", "--all-targets"])


@angreal.command(name="check", about="CI gate: rustfmt --check then clippy")
def check():
    rc = run(["cargo", "fmt", "--", "--check"])
    if rc != 0:
        return rc
    return run(["cargo", "clippy", "--all-targets"])
