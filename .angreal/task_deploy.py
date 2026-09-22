"""Deploy tasks: build & run the containerized home server (deploy/docker-compose.yml, SQUIRE-T-0129).

The stack is compose project ``squire-home`` (pinned via ``name:`` in the file), so every command
here targets the same containers regardless of the working directory.

SAFETY: never ``down -v`` on this stack — the ``squire-data`` volume IS the household (the database
plus the signing key every paired phone's token was minted with).
"""

import subprocess
import sys
import time
import urllib.request
from pathlib import Path

import angreal

from utils import root, run

# angreal exits on a non-zero return WITHOUT flushing Python's stdout, which is block-buffered when
# piped (CI, an agent, `| tee`). The messages that matter most — a refusal, a failed health wait —
# are printed right before exactly those returns, and were silently lost. Line-buffer instead.
sys.stdout.reconfigure(line_buffering=True)

deploy = angreal.command_group(
    name="deploy",
    about="Build & run the containerized home server (deploy/docker-compose.yml)",
)

DEPLOY_DIR = root() / "deploy"
COMPOSE_FILE = DEPLOY_DIR / "docker-compose.yml"
ENV_FILE = DEPLOY_DIR / ".env"
SERVICE = "squire"

# Topology note: the server and the tailscale node both JOIN the `net` holder's network namespace
# (see docker-compose.yml). Because neither joins the OTHER, either can be restarted or recreated
# without orphaning its sibling — so nothing here re-attaches sidecars. (The first version had
# tailscale join the server; a stop/start of the server silently took the tailnet path down.) The
# one rule: never recreate `net` on its own. A plain `up -d` is always safe and self-heals.


def _env(key, default=None):
    """A value from ``deploy/.env`` (what compose itself will interpolate), else ``default``."""
    try:
        for line in ENV_FILE.read_text().splitlines():
            line = line.strip()
            if line.startswith(f"{key}="):
                value = line.split("=", 1)[1].strip().strip('"').strip("'")
                if value:
                    return value
    except OSError:
        pass
    return default


def _health_url():
    return f"http://127.0.0.1:{_env('SQUIRE_API_PORT', '8080')}/health"


def _compose_argv(args):
    # Explicit about the env file and project directory rather than relying on compose's implicit
    # resolution, so the stack comes up identically from any working directory.
    cmd = ["docker", "compose", "--project-directory", str(DEPLOY_DIR)]
    if ENV_FILE.exists():
        cmd += ["--env-file", str(ENV_FILE)]
    return cmd + ["-f", str(COMPOSE_FILE)] + list(args)


def _compose(args):
    return run(_compose_argv(args), cwd=DEPLOY_DIR)


def _compose_out(args):
    got = subprocess.run(_compose_argv(args), cwd=str(DEPLOY_DIR), capture_output=True, text=True)
    return got.stdout.strip()


def _require_env():
    if ENV_FILE.exists():
        return True
    print(
        "deploy/.env is missing — `cp deploy/.env.example deploy/.env` and set SQUIRE_PAIR_HOST.",
        file=sys.stderr,
    )
    return False


def _wait_healthy(timeout_s=90):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(_health_url(), timeout=3) as r:
                if r.status == 200:
                    return True
        except Exception:
            pass
        time.sleep(2)
    return False


def _report_up():
    print("waiting for the server to be healthy ...")
    if not _wait_healthy():
        print("squire did not become healthy in time — check `angreal deploy logs`")
        return 1
    print(f"squire healthy — phone API on :{_env('SQUIRE_API_PORT', '8080')}, "
          f"the Keep at http://127.0.0.1:{_env('SQUIRE_KEEP_PORT', '4920')}")
    return 0


@deploy()
@angreal.command(
    name="up",
    about="Bring the stack up (detached), then wait for the server to be healthy",
    tool=angreal.ToolDescription(
        """
        Start the containerized home server (and the Tailscale node, when the `tailscale`
        compose profile is enabled in deploy/.env) detached, then wait for the API's /health.

        ## When to use
        - After `angreal deploy down`, or on a fresh machine, to run the server in Docker.

        ## Notes
        - Needs deploy/.env (copy deploy/.env.example; SQUIRE_PAIR_HOST is required).
        - A bare-metal squire-serve (launchd) on the same ports must be stopped first — the
          port publish fails otherwise, and two servers must never share one data dir.
        - Builds a missing image; `--build` forces a rebuild first.
        """,
        risk_level="safe",
    ),
)
@angreal.argument(
    name="build", long="build", is_flag=True, takes_value=False,
    help="Rebuild the image before starting",
)
def up(build=False):
    if not _require_env():
        return 1
    args = ["up", "-d"]
    if build:
        args.append("--build")
    if _compose(args) != 0:
        return 1
    return _report_up()


@deploy()
@angreal.command(
    name="down",
    about="Stop & remove the stack's containers (KEEPS volumes — the household is safe)",
)
def down():
    # Deliberately NO `-v`: the squire-data volume is the household.
    return _compose(["--profile", "tailscale", "down"])


@deploy()
@angreal.command(name="build", about="Build the squire-serve image")
def build():
    return _compose(["build", SERVICE])


@deploy()
@angreal.command(
    name="redeploy",
    about="Rebuild the image and roll it onto the running stack",
    tool=angreal.ToolDescription(
        """
        The deploy-a-code-change flow: rebuild the image, `up -d` (recreates the server off the new
        image and applies any .env change; the Tailscale node is left running), wait for health.

        ## When to use
        - After changing server code, to roll it onto the running stack. The image has
          self-update OFF — this IS how a containerized server is updated.

        ## Notes
        - Data lives in the squire-data volume and survives the recreate.
        - `--prune` also drops the Docker build cache afterwards (repeated image builds fill
          the Docker Desktop VM disk); the next build is then a cold one.
        """,
        risk_level="safe",
    ),
)
@angreal.argument(
    name="prune", long="prune", is_flag=True, takes_value=False,
    help="Prune the Docker builder cache afterwards",
)
def redeploy(prune=False):
    if not _require_env():
        return 1
    if _compose(["build", SERVICE]) != 0:
        return 1
    # Plain `up -d`: recreates the server (its image changed) and anything whose config in .env
    # changed, leaves the `net` holder alone, and heals any stale namespace reference.
    if _compose(["up", "-d"]) != 0:
        return 1
    if prune:
        run(["docker", "builder", "prune", "-af"])
    return _report_up()


@deploy()
@angreal.command(name="logs", about="Tail a service's logs (default: squire; -s tailscale)")
@angreal.argument(
    name="service", long="service", short="s", takes_value=True,
    help="Service to tail (default: squire)",
)
def logs(service=None):
    return _compose(["--profile", "tailscale", "logs", "-f", "--tail=100", service or SERVICE])


@deploy()
@angreal.command(name="status", about="Show container status, server health, and the tailnet node")
def status():
    _compose(["--profile", "tailscale", "ps"])
    running = bool(_compose_out(["ps", "-q", "--status", "running", SERVICE]))
    try:
        with urllib.request.urlopen(_health_url(), timeout=3) as r:
            print(f"\nsquire health: HTTP {r.status} ({_health_url()})")
            if not running:
                # The probe is by PORT. A bare-metal squire-serve on the same port answers it too.
                print("  ^ NOT this stack — its server container is not running; something else "
                      "(a bare-metal squire-serve?) is answering on that port.")
    except Exception as e:
        print(f"\nsquire health: unreachable ({e})")
    if _compose_out(["--profile", "tailscale", "ps", "-q", "tailscale"]):
        print("\ntailnet:")
        _compose(["--profile", "tailscale", "exec", "tailscale", "tailscale", "status", "--peers=false"])
    return 0


def _default_data_dir():
    """Where a bare-metal squire-serve keeps its data (`dirs::data_dir()/squire`)."""
    if sys.platform == "darwin":
        return Path.home() / "Library" / "Application Support" / "squire"
    return Path.home() / ".local" / "share" / "squire"


@deploy()
@angreal.command(
    name="import-data",
    about="Copy an existing bare-metal data dir (household db + signing key) into the stack's volume",
    tool=angreal.ToolDescription(
        """
        One-time migration of a bare-metal install into the container: COPIES the household
        database, signing.key and the OTA updates dir into the squire-data volume. The source is
        never modified, so the bare-metal install remains a fallback.

        ## When to use
        - Once, when moving a running household from launchd/bare-metal into Docker. Keeping
          signing.key is what keeps every already-paired phone signed in.

        ## Preconditions (enforced)
        - No bare-metal squire-serve process running (a live SQLite file cannot be copied safely).
        - The stack's server container is not running.
        - The volume holds no household yet, unless `--force`.
        """,
        risk_level="destructive",
    ),
)
@angreal.argument(
    name="source", long="from", takes_value=True,
    help="Data dir to import (default: this OS's bare-metal squire data dir)",
)
@angreal.argument(
    name="force", long="force", is_flag=True, takes_value=False,
    help="Overwrite a household already in the volume",
)
def import_data(source=None, force=False):
    if not _require_env():
        return 1
    src = Path(source).expanduser() if source else _default_data_dir()
    if not (src / "signing.key").is_file() or not list(src.glob("*.sqlite")):
        print(f"{src} does not look like a squire data dir (need signing.key + <household>.sqlite)")
        return 1

    if subprocess.run(["pgrep", "-x", "squire-serve"], capture_output=True).returncode == 0:
        print(
            "a bare-metal squire-serve is running — stop it first; copying a live SQLite file can\n"
            "yield a torn database, and launchd (KeepAlive) restarts a merely-killed one:\n"
            "  launchctl list | grep -i squire                     # find your install's label\n"
            "  launchctl bootout gui/$(id -u)/<label>              # unload it now\n"
            "  launchctl disable gui/$(id -u)/<label>              # and keep it from returning at login"
        )
        return 1
    if _compose_out(["ps", "-q", "--status", "running", SERVICE]):
        print("the stack's server is running — `angreal deploy down` first")
        return 1

    # Runs in the server image itself (as root, to chown for the image's uid) against the service's
    # own volume mounts, so the volume is created with compose's labels and nothing else is pulled.
    # NOT `--no-deps`: the server joins the `net` holder's namespace, and on a fresh machine compose
    # must be allowed to start that holder ("cannot share network namespace … container missing").
    # --force must CLEAR the old database first, sidecars included: a `-wal`/`-shm` left over from
    # the replaced database, sitting next to the imported one, is how SQLite files get corrupted.
    guard = "rm -f /data/*.sqlite /data/*.sqlite-wal /data/*.sqlite-shm; " if force else (
        'if ls /data/*.sqlite >/dev/null 2>&1; then '
        'echo "the volume already holds a household — pass --force to overwrite it"; exit 3; fi; '
    )
    script = guard + "cp -a /src/. /data/ && chown -R squire:squire /data && ls -la /data"
    rc = _compose([
        "run", "--rm", "--user", "0", "--entrypoint", "sh",
        "-v", f"{src}:/src:ro", SERVICE, "-c", script,
    ])
    if rc == 0:
        print(f"\nimported {src} → the squire-data volume. Next: `angreal deploy up`")
    return rc


def _run_reset(env_pairs):
    """Run the image's `reset_secret` in a one-off container against the service's own volume."""
    args = ["run", "--rm", "--entrypoint", "/usr/local/bin/reset_secret"]
    for k, v in env_pairs:
        args += ["-e", f"{k}={v}"]
    return _compose(args + [SERVICE])


@deploy()
@angreal.command(
    name="reset-secret",
    about="Forgot a password: reset a member's login secret in the stack's household (or list members)",
    tool=angreal.ToolDescription(
        """
        The recovery escape hatch for the containerized household. With no `--user` it LISTS the
        members (read-only, server keeps running). With `--user` it stops the server, replaces that
        member's credential hash (same UserId — paired devices and history are untouched), and
        starts the server again (~10s of downtime).

        ## Notes
        - Squire has no in-app "change my secret"; this is the only recovery path.
        - `--user` takes an id or a display name (case-insensitive, must match exactly one member).
        - Without `--secret` a random one is generated and printed ONCE. Minimum 8 characters.
        - Resetting the WRONG member locks that person out — list first.
        """,
        risk_level="destructive",
    ),
)
@angreal.argument(name="user", long="user", takes_value=True, help="Member id or display name to reset")
@angreal.argument(name="secret", long="secret", takes_value=True, help="New secret (default: generate one)")
def reset_secret(user=None, secret=None):
    if not _require_env():
        return 1
    if not user:
        return _run_reset([])

    import secrets as _secrets
    generated = secret is None
    if generated:
        # ~77 bits, typeable on a phone: five groups of five from an unambiguous alphabet.
        alphabet = "abcdefghjkmnpqrstuvwxyz23456789"
        secret = "-".join("".join(_secrets.choice(alphabet) for _ in range(5)) for _ in range(5))

    was_running = bool(_compose_out(["ps", "-q", "--status", "running", SERVICE]))
    if was_running:
        # Single SQLite writer: the server must not hold the household while the hash is replaced.
        print("stopping the server for the reset ...")
        if _compose(["stop", SERVICE]) != 0:
            return 1
    rc = _run_reset([("RESET_USER", user), ("NEW_SECRET", secret)])
    if was_running:
        print("starting the server again ...")
        # Safe for the tailnet path: the namespace belongs to the `net` holder, not to the server.
        if _compose(["start", SERVICE]) != 0 or _report_up() != 0:
            return 1
    if rc == 0 and generated:
        print(f"\nnew secret for '{user}':  {secret}\n(shown once — it is stored only as a hash)")
    return rc

