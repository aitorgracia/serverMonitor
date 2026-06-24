#!/usr/bin/env bash
set -euo pipefail

APP_DIR="$(cd "$(dirname "$0")" && pwd)"
BIN="$APP_DIR/monitor_dashboard.sh"
ICON_SRC="$APP_DIR/icon.svg"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
DESKTOP_DIR="$HOME/.local/share/applications"

mkdir -p "$ICON_DIR" "$DESKTOP_DIR"

# Install icon
if [ -f "$ICON_SRC" ]; then
    cp "$ICON_SRC" "$ICON_DIR/monitor_dashboard.svg"
    echo "✅ Icono instalado"
    gtk-update-icon-cache "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

# Generate .desktop with absolute paths
cat > "$DESKTOP_DIR/monitor_dashboard.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Monitor del Servidor
Comment=Monitorización del servidor con métricas en tiempo real
Exec=$BIN
Icon=utilities-system-monitor
Terminal=false
Categories=System;Monitor;Utility;
StartupNotify=true
EOF

echo "✅ Lanzador instalado en $DESKTOP_DIR/monitor_dashboard.desktop"
echo "🔍 Busca 'Monitor del Servidor' en tu menú de aplicaciones"
echo ""
echo "📌 Para desinstalar:"
echo "   rm $DESKTOP_DIR/monitor_dashboard.desktop"
echo "   rm -f $ICON_DIR/monitor_dashboard.svg"
