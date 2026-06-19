"""Run tasks: the persistent home server, the throwaway demo, stop, and the admin escape hatch.

Ports + admin + pairing host come from the environment (see
``crates/squire-home/src/bin/squire-serve.rs``): ``API_PORT`` (8080), ``KEEP_PORT`` (4920),
``SQUIRE_PAIR_HOST``, ``SQUIRE_ADMIN_NAME`` / ``SQUIRE_ADMIN_SECRET`` (secret REQUIRED on first run),
``SQUIRE_APK_DIR`` (defaults to ``<data_dir>/updates``). No secrets are baked into these tasks.
"""

import angreal

from utils import bash, run


@angreal.command(
    name="serve",
    about="Run the persistent home server (squire-serve); configure via env, --release for prod",
)
@angreal.argument(
    name="release", long="release", is_flag=True, takes_value=False,
    help="Run the optimized release build (what prod uses)",
)
def serve(release=False):
    cmd = ["cargo", "run"]
    if release:
        cmd.append("--release")
    cmd += ["-p", "squire-home", "--bin", "squire-serve"]
    return run(cmd)


@angreal.command(name="demo", about="Run the throwaway demo server (squire-home; wiped+reseeded each start)")
def demo():
    return run(["cargo", "run", "-p", "squire-home", "--bin", "squire-home"])


@angreal.command(name="stop", about="Stop a running server by freeing its TCP ports")
@angreal.argument(
    name="ports", long="ports", takes_value=True,
    help="Comma-separated listen ports to free (default: 8088,4920)",
)
def stop(ports="8088,4920"):
    return bash(
        f"pids=$(lsof -tiTCP:{ports} -sTCP:LISTEN 2>/dev/null); "
        f"if [ -n \"$pids\" ]; then echo \"$pids\" | xargs kill -TERM; echo 'sent SIGTERM'; "
        f"else echo '(nothing listening on {ports})'; fi"
    )


@angreal.command(
    name="add-admin",
    about="Escape hatch: insert a Knight admin into the live household (run with the server stopped)",
)
@angreal.argument(name="name", long="name", takes_value=True, help="Admin display name (default: Dad)")
@angreal.argument(name="secret", long="secret", takes_value=True, help="Initial login secret (REQUIRED)")
@angreal.argument(
    name="acting", long="acting-knight", takes_value=True,
    help="Existing Knight UserId that authorises the add (default: 2)",
)
def add_admin(name="Dad", secret=None, acting="2"):
    if not secret:
        print("error: --secret is required")
        return 1
    env = {"ADMIN_NAME": name, "ADMIN_SECRET": secret, "ACTING_KNIGHT": acting}
    return run(["cargo", "run", "-p", "squire-home", "--bin", "add_admin"], env=env)
