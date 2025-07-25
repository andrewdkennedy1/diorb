#!/usr/bin/env powershell

# Test script for DIORB application
Write-Host "Testing DIORB Disk Benchmark Application" -ForegroundColor Green

# Build the application
Write-Host "Building application..." -ForegroundColor Yellow
cargo build --release

if ($LASTEXITCODE -eq 0) {
    Write-Host "Build successful!" -ForegroundColor Green
    
    # Test basic functionality
    Write-Host "Application built successfully. You can now run:" -ForegroundColor Cyan
    Write-Host "  cargo run" -ForegroundColor White
    Write-Host ""
    Write-Host "Expected flow:" -ForegroundColor Yellow
    Write-Host "  1. Disk selection screen with detected drives" -ForegroundColor White
    Write-Host "  2. Press Enter to start 1GB speed test" -ForegroundColor White
    Write-Host "  3. Press C to configure benchmark settings" -ForegroundColor White
    Write-Host "  4. Running screen with real-time progress" -ForegroundColor White
    Write-Host "  5. Results screen with detailed metrics" -ForegroundColor White
    Write-Host ""
    Write-Host "Controls:" -ForegroundColor Yellow
    Write-Host "  ↑↓ - Navigate" -ForegroundColor White
    Write-Host "  Enter - Select/Start Test" -ForegroundColor White
    Write-Host "  C - Configuration" -ForegroundColor White
    Write-Host "  S - Start Test (from config)" -ForegroundColor White
    Write-Host "  Q/Esc - Quit/Back" -ForegroundColor White
} else {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}