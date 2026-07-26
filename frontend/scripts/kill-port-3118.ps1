# Kill any process occupying port 3118
Get-NetTCPConnection -LocalPort 3118 -ErrorAction SilentlyContinue | ForEach-Object {
    Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue
}
