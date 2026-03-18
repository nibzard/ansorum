#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")/.."

if rg -n "href=\"\{\{ get_url\(path='@/themes/_index.md'\) \}\"" docs/templates/index.html >/dev/null; then
  echo "Root docs navigation must not link directly to the inherited theme gallery."
  exit 1
fi

if rg -n "<h1>Zola themes</h1>|<h1>Zola themes in" docs/templates/themes.html docs/templates/theme-tags/single.html >/dev/null; then
  echo "Theme templates must stay explicitly archived and must not restore the old active Zola gallery headings."
  exit 1
fi

if rg -n -F "| Zola {% endblock title %}" docs/templates/theme.html >/dev/null; then
  echo "Theme detail pages must not use the inherited Zola title branding."
  exit 1
fi

echo "Docs product-surface checks passed."
