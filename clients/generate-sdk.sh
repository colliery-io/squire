#!/usr/bin/env bash
# Generate the Kotlin client SDK for the phone apps (the Squire, S-0005; the Knight, S-0006) from
# the frozen OpenAPI contract emitted by the Rust API (ADR SQUIRE-A-0009 / SQUIRE-T-0031).
#
# The contract `crates/api/openapi.json` is the single source of truth — regenerate it from the
# Rust types with `cargo run -p api --example gen_openapi` (or `UPDATE_OPENAPI=1 cargo test -p api
# --test openapi`). This script then turns that spec into a Kotlin SDK with openapi-generator.
#
# Requirements: a JVM (Java 17+). The openapi-generator CLI jar is fetched on first run into
# `tools/` (gitignored). No Android SDK is needed for SDK generation.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SPEC="$ROOT/crates/api/openapi.json"
OUT="$ROOT/clients/squire-sdk"
JAR="$ROOT/tools/openapi-generator-cli.jar"
GEN_VERSION="7.11.0"
JAR_URL="https://repo1.maven.org/maven2/org/openapitools/openapi-generator-cli/${GEN_VERSION}/openapi-generator-cli-${GEN_VERSION}.jar"

[ -f "$SPEC" ] || { echo "missing $SPEC — run: cargo run -p api --example gen_openapi" >&2; exit 1; }

if [ ! -f "$JAR" ]; then
  echo "fetching openapi-generator-cli $GEN_VERSION → $JAR"
  mkdir -p "$ROOT/tools"
  curl -fsSL -o "$JAR" "$JAR_URL"
fi

echo "generating Kotlin SDK → $OUT"
rm -rf "$OUT"
java -jar "$JAR" generate \
  -i "$SPEC" \
  -g kotlin \
  --library jvm-okhttp4 \
  --additional-properties=serializationLibrary=kotlinx_serialization,packageName=com.squire.sdk,apiPackage=com.squire.sdk.api,modelPackage=com.squire.sdk.model,artifactId=squire-sdk \
  -o "$OUT"

echo "done. Models: $(ls "$OUT/src/main/kotlin/com/squire/sdk/model" | wc -l | tr -d ' '); APIs: SquireApi, KnightApi, ControlApi"
