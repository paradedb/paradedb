"""Rebuild the website MCP after a successful docs deployment."""

import argparse
import json
import os
import time
import urllib.error
import urllib.parse
import urllib.request

REPOSITORY = "paradedb/paradedb"
MINTLIFY_APP_ID = 222410
DOCS_URL = "https://www.paradedb.com/docs"


def required_env(name):
    """Read a required setting without logging its value."""
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"Missing {name}")
    return value


def request_json(service, url, *, method="GET", token=None):
    """Call an API, retrying reads and keeping credentials out of errors."""
    headers = {"Accept": "application/json", "User-Agent": "paradedb-docs-refresh"}
    if method == "POST":
        headers["Content-Type"] = "application/json"
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(
        url, headers=headers, method=method, data=b"" if method == "POST" else None
    )
    for attempt in range(3):
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                result = json.load(response)
            if not isinstance(result, dict):
                raise TypeError(f"{service} returned an invalid response")
            return result
        except urllib.error.HTTPError as error:
            if (
                method == "GET"
                and attempt < 2
                and (error.code == 429 or error.code >= 500)
            ):
                time.sleep(5)
                continue
            raise RuntimeError(f"{service} returned HTTP {error.code}") from None
        except (urllib.error.URLError, TimeoutError):
            if method == "GET" and attempt < 2:
                time.sleep(5)
                continue
            # Deploy hook URLs contain credentials; don't print network exceptions.
            raise RuntimeError(f"{service} request failed") from None
        except (ValueError, UnicodeError):
            raise RuntimeError(f"{service} returned invalid JSON") from None


def deploy_hook():
    """Validate the configured Vercel hook."""
    url = required_env("WEBSITE_DEPLOY_HOOK_URL")
    parsed = urllib.parse.urlsplit(url)
    if (
        parsed.scheme != "https"
        or parsed.netloc != "api.vercel.com"
        or not parsed.path.startswith("/v1/integrations/deploy/")
    ):
        raise RuntimeError("WEBSITE_DEPLOY_HOOK_URL must be a Vercel deploy hook")
    return url


def refresh_website():
    """Queue a website build."""
    response = request_json("Vercel", deploy_hook(), method="POST")
    job = response.get("job") or {}
    if (
        not isinstance(job, dict)
        or not job.get("id")
        or job.get("state") in ("ERROR", "CANCELED")
    ):
        raise RuntimeError("Vercel did not queue a website rebuild")
    print("Website rebuild queued.")


def is_production_check(event):
    """Accept successful Mintlify production checks on main."""
    check = event.get("check_run") or {}
    app = check.get("app") or {}
    if not (
        event.get("action") == "completed"
        and event.get("repository", {}).get("full_name") == REPOSITORY
        and app.get("id") == MINTLIFY_APP_ID
        and app.get("slug") == "mintlify"
        and check.get("name") == "Mintlify Deployment"
        and check.get("conclusion") == "success"
        and DOCS_URL in ((check.get("output") or {}).get("summary") or "")
    ):
        return False

    suite_id = check.get("check_suite", {}).get("id")
    if not isinstance(suite_id, int):
        raise TypeError("Mintlify check is missing its check suite")
    suite = request_json(
        "GitHub",
        f"https://api.github.com/repos/{REPOSITORY}/check-suites/{suite_id}",
        token=required_env("GH_TOKEN"),
    )
    return (
        suite.get("app", {}).get("id") == MINTLIFY_APP_ID
        and suite.get("head_branch") == "main"
        and bool(check.get("head_sha"))
        and suite.get("head_sha") == check["head_sha"]
    )


def publish_docs():
    """Publish docs and wait for success before rebuilding the website."""
    deploy_hook()
    project = urllib.parse.quote(required_env("MINTLIFY_PROJECT_ID"), safe="")
    token = required_env("MINTLIFY_API_KEY")
    response = request_json(
        "Mintlify",
        f"https://api.mintlify.com/v1/project/update/{project}",
        method="POST",
        token=token,
    )
    status_id = response.get("statusId")
    if not isinstance(status_id, str) or not status_id:
        raise RuntimeError("Mintlify did not return a deployment status ID")
    status_url = (
        "https://api.mintlify.com/v1/project/update-status/"
        + urllib.parse.quote(status_id, safe="")
    )
    deadline = time.monotonic() + 900
    previous = None
    while time.monotonic() < deadline:
        status = request_json("Mintlify", status_url, token=token).get("status")
        if status == "success":
            print("Mintlify deployment succeeded.")
            refresh_website()
            return
        if status == "failure":
            raise RuntimeError("Mintlify deployment failed; website was not rebuilt")
        if status not in ("queued", "in_progress"):
            raise RuntimeError("Mintlify returned an unknown deployment status")
        if status != previous:
            print(f"Mintlify deployment: {status}", flush=True)
            previous = status
        time.sleep(10)
    raise RuntimeError("Mintlify deployment timed out; website was not rebuilt")


def main():
    """Handle deployment checks, publishing, and manual refreshes."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("check", "publish", "refresh"))
    mode = parser.parse_args().mode
    if mode == "publish":
        publish_docs()
    elif mode == "check":
        with open(required_env("GITHUB_EVENT_PATH"), encoding="utf-8") as event_file:
            event = json.load(event_file)
        if is_production_check(event):
            refresh_website()
        else:
            print("Not a successful production docs deployment; skipping.")
    else:
        refresh_website()


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, TypeError) as error:
        raise SystemExit(str(error)) from None
