#!/usr/bin/env python3
"""Emit a secret-free operational audit of one or more Synapse YAML configs."""

from __future__ import annotations

import argparse
import ipaddress
import json
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

import yaml


def merge(left: dict[str, Any], right: dict[str, Any]) -> dict[str, Any]:
    result = dict(left)
    for key, value in right.items():
        if isinstance(value, dict) and isinstance(result.get(key), dict):
            result[key] = merge(result[key], value)
        else:
            result[key] = value
    return result


def configured(value: Any) -> bool:
    return value not in (None, "", [], {})


def public_binding(address: str) -> bool:
    return address in ("0.0.0.0", "::", "")


def blacklist_covers(entries: list[str], required: str) -> bool:
    required_network = ipaddress.ip_network(required)
    for entry in entries:
        try:
            if required_network.subnet_of(ipaddress.ip_network(entry, strict=False)):
                return True
        except ValueError:
            continue
    return False


def audit(configuration: dict[str, Any], config_count: int) -> dict[str, Any]:
    findings: list[dict[str, str]] = []

    def add(severity: str, code: str, message: str) -> None:
        findings.append({"severity": severity, "code": code, "message": message})

    database = configuration.get("database") or {}
    database_args = database.get("args") or {}
    listeners = configuration.get("listeners") or []
    listener_facts = []
    for listener in listeners:
        if not isinstance(listener, dict):
            continue
        configured_bindings = listener.get("bind_addresses")
        if isinstance(configured_bindings, str):
            bindings = [configured_bindings]
        else:
            bindings = configured_bindings or ["0.0.0.0", "::"]
        resources = sorted(
            {
                name
                for resource in listener.get("resources") or []
                if isinstance(resource, dict)
                for name in resource.get("names") or []
            }
        )
        listener_type = listener.get("type", "http")
        binds_publicly = any(public_binding(str(address)) for address in bindings)
        listener_facts.append(
            {
                "type": listener_type,
                "port": listener.get("port"),
                "bindings": bindings,
                "resources": resources,
                "tls": bool(listener.get("tls", False)),
                "x_forwarded": bool(listener.get("x_forwarded", False)),
            }
        )
        if listener_type in ("metrics", "manhole", "replication") and binds_publicly:
            add(
                "error",
                f"public_{listener_type}_listener",
                f"The {listener_type} listener on port {listener.get('port')} binds publicly.",
            )
        for sensitive_resource in sorted(
            set(resources).intersection({"metrics", "replication"})
        ):
            if listener_type == "http" and binds_publicly:
                add(
                    "error",
                    f"public_{sensitive_resource}_resource",
                    f"The HTTP {sensitive_resource} resource on port {listener.get('port')} binds publicly.",
                )
        if (
            listener_type == "http"
            and not listener.get("tls", False)
            and not listener.get("x_forwarded", False)
            and set(resources).intersection({"client", "federation"})
        ):
            add(
                "warning",
                "proxy_headers",
                f"Plain HTTP listener {listener.get('port')} does not trust proxy headers; verify it is not behind a reverse proxy.",
            )

    if database.get("name") != "psycopg2":
        add("error", "database", "Production Synapse should use PostgreSQL (psycopg2), not SQLite.")

    if configuration.get("enable_registration_without_verification", False):
        add(
            "error",
            "open_registration",
            "Registration without verification is enabled.",
        )

    preview_blacklist = configuration.get("url_preview_ip_range_blacklist") or []
    if configuration.get("url_preview_enabled", False):
        required_ranges = [
            "127.0.0.0/8",
            "10.0.0.0/8",
            "172.16.0.0/12",
            "192.168.0.0/16",
            "169.254.0.0/16",
            "::1/128",
            "fc00::/7",
            "fe80::/10",
        ]
        missing = [item for item in required_ranges if not blacklist_covers(preview_blacklist, item)]
        if missing:
            add(
                "error",
                "url_preview_ssrf",
                "URL previews do not blacklist every loopback, private, link-local, and unique-local range.",
            )

    worker_configured = configured(configuration.get("worker_app")) or configured(
        configuration.get("instance_map")
    )
    redis = configuration.get("redis") or {}
    if worker_configured and not redis.get("enabled", False):
        add("error", "worker_redis", "Worker topology is configured without enabled Redis replication.")
    if worker_configured and not configured(configuration.get("worker_replication_secret")):
        add("warning", "replication_secret", "Workers have no configured replication secret.")

    turn_uris = configuration.get("turn_uris") or []
    if not turn_uris:
        add("warning", "turn", "No TURN URIs are configured; calls will be unreliable across NAT/firewalls.")

    public_baseurl = configuration.get("public_baseurl")
    if public_baseurl and urlparse(public_baseurl).scheme != "https":
        add("error", "public_baseurl", "public_baseurl must use HTTPS in production.")

    authentication = configuration.get("matrix_authentication_service") or {}
    facts = {
        "config_file_count": config_count,
        "server_name": configuration.get("server_name"),
        "public_baseurl_host": urlparse(public_baseurl).hostname if public_baseurl else None,
        "database": {
            "engine": database.get("name"),
            "pool_min": database_args.get("cp_min"),
            "pool_max": database_args.get("cp_max"),
        },
        "listeners": listener_facts,
        "registration": {
            "enabled": bool(configuration.get("enable_registration", False)),
            "without_verification": bool(
                configuration.get("enable_registration_without_verification", False)
            ),
            "shared_secret_configured": configured(
                configuration.get("registration_shared_secret")
            ),
        },
        "url_previews": {
            "enabled": bool(configuration.get("url_preview_enabled", False)),
            "blacklist_entry_count": len(preview_blacklist),
        },
        "metrics_enabled": bool(configuration.get("enable_metrics", False)),
        "workers": {
            "configured": worker_configured,
            "instance_count": len(configuration.get("instance_map") or {}),
            "redis_enabled": bool(redis.get("enabled", False)),
            "replication_secret_configured": configured(
                configuration.get("worker_replication_secret")
            ),
        },
        "authentication_service": {
            "enabled": bool(authentication.get("enabled", False)),
            "secret_configured": configured(authentication.get("secret"))
            or configured(authentication.get("secret_path")),
        },
        "turn": {
            "uri_count": len(turn_uris),
            "shared_secret_configured": configured(
                configuration.get("turn_shared_secret")
            ),
        },
        "findings": findings,
    }
    return facts


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Audit Synapse YAML without emitting secret values."
    )
    parser.add_argument("configs", nargs="+", type=Path)
    arguments = parser.parse_args()

    combined: dict[str, Any] = {}
    for config_path in arguments.configs:
        loaded = yaml.safe_load(config_path.read_text(encoding="utf-8")) or {}
        if not isinstance(loaded, dict):
            raise SystemExit(f"{config_path}: top-level YAML must be an object")
        combined = merge(combined, loaded)

    print(json.dumps(audit(combined, len(arguments.configs)), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
