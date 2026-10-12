"""Explicitly probe a user-authorized private API file; never discover old credentials.

Development smoke only. This does not replace native Registry, Pi, tools or release gates.
"""
import argparse
import concurrent.futures
import json
import os
from pathlib import Path
import stat
import time
import urllib.error
import urllib.parse
import urllib.request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    info = args.config.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
        parser.error("config must be an owned regular file with private permissions")
    config = json.loads(args.config.read_text())
    endpoint = config["base_url"].rstrip("/")
    url = urllib.parse.urlsplit(endpoint)
    if url.scheme != "https" or not url.hostname or url.username or url.password or url.query or url.fragment:
        parser.error("config requires a plain HTTPS API endpoint")
    models = config["models"]
    if not isinstance(models, list) or not 1 <= len(models) <= 2 or not all(isinstance(model, str) and 0 < len(model) <= 128 for model in models):
        parser.error("provide one or two explicit authorized model names")
    key = config["api_key"]
    if not isinstance(key, str) or not key or "\n" in key or "\r" in key:
        parser.error("invalid credential")

    class SameOriginRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, request, fp, code, message, headers, new_url):
            target = urllib.parse.urlsplit(new_url)
            if (target.scheme, target.hostname, target.port) != (url.scheme, url.hostname, url.port):
                raise urllib.error.HTTPError(request.full_url, code, "cross-origin redirect rejected", headers, fp)
            return super().redirect_request(request, fp, code, message, headers, new_url)

    opener = urllib.request.build_opener(SameOriginRedirect())

    def request(path, body=None):
        data = None if body is None else json.dumps(body).encode()
        frame = urllib.request.Request(endpoint + path, data=data, headers={
            "Authorization": "Bearer " + key, "Accept": "application/json", "Content-Type": "application/json",
        })
        with opener.open(frame, timeout=45) as response:
            wire = response.read(65537)
            if len(wire) > 65536:
                raise ValueError("response over development smoke limit")
            return response.status, json.loads(wire)

    catalog = {}
    try:
        status, data = request("/models")
        ids = {item.get("id") for item in data.get("data", []) if isinstance(item, dict)}
        catalog = {"http_status": status, "requested_models": {model: model in ids for model in models}}
    except urllib.error.HTTPError as error:
        catalog["http_status"] = error.code
    except Exception as error:
        catalog["error_type"] = type(error).__name__

    def probe(model):
        started = time.monotonic()
        body = {"model": model, "messages": [{"role": "user", "content": "Return only the single ASCII digit equal to 2+2."}], "max_completion_tokens": 64, "stream": False}
        result = {"model": model, "protocol": "openai_chat_completions", "request": body}
        try:
            status, data = request("/chat/completions", body)
            choice = data.get("choices", [{}])[0]
            content = choice.get("message", {}).get("content")
            matched = isinstance(content, str) and content.strip() == "4"
            result.update(http_status=status, expected_digit_received=matched)
            if matched:
                result["response_content"] = content
        except urllib.error.HTTPError as error:
            result["http_status"] = error.code
        except Exception as error:
            result["error_type"] = type(error).__name__
        result["elapsed_ms"] = round((time.monotonic() - started) * 1000, 3)
        return result

    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
        results = list(executor.map(probe, models))
    report = {"endpoint": endpoint, "scope": "authorized development transport/short-text smoke only", "catalog": catalog, "models": results}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))
    return 0 if all(result.get("http_status") == 200 and result.get("expected_digit_received") for result in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
