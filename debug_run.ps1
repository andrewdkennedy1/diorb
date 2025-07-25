#!/usr/bin/env powershell

Write-Host "Building and running DIORB with debug output..." -ForegroundColor Green
cargo build

if ($LASTEXITCODE -eq 0) {
    Write-Host "Starting application - watch for debug output..." -ForegroundColor Yellow
    Write-Host "Press Ctrl+C to stop if it goes into benchmark immediately" -ForegroundColor Red
    Write-Host ""
    
    # Run with timeout to prevent hanging
    $job = Start-Job -ScriptBlock { 
        Set-Location $using:PWD
        cargo run 2>&1
    }
    
    # Wait for 5 seconds to see initial output
    Wait-Job $job -Timeout 5
    
    if ($job.State -eq "Running") {
        Write-Host "Application is still running - stopping to check output..." -ForegroundColor Yellow
        Stop-Job $job
    }
    
    $output = Receive-Job $job
    Remove-Job $job
    
    Write-Host "Application output:" -ForegroundColor Cyan
    $output | ForEach-Object { Write-Host $_ }
} else {
    Write-Host "Build failed!" -ForegroundColor Red
}