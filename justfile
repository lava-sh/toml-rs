set windows-shell := ["pwsh.exe", "-NoLogo", "-NoProfile", "-Command"]

alias i := install

WHEEL_DIR := "wheel/"

[private]
@default:
    just --list

[windows]
[script("pwsh.exe", "-NoLogo", "-NoProfile", "-Command")]
[doc("Build Python wheel with mimalloc")]
install:
    $ErrorActionPreference = "Stop"

    if (Test-Path {{ WHEEL_DIR }}) {
        Remove-Item {{ WHEEL_DIR }} -Recurse -Force
    }

    .\.venv\Scripts\Activate.ps1

    maturin build --out {{ WHEEL_DIR }} --release --features mimalloc

    $wheel = Get-ChildItem {{ WHEEL_DIR }}/*.whl | Select-Object -First 1

    if (Get-Command uv -ErrorAction SilentlyContinue) {
        Write-Host "uv found, using uv"
        uv pip install $wheel.FullName --force-reinstall
    } else {
        Write-Host "uv not found, using pip"
        pip install $wheel.FullName --force-reinstall
    }

[doc("Bump deps & gitHub actions")]
[script("pwsh.exe", "-NoLogo", "-NoProfile", "-Command")]
[windows]
bump:
    $ErrorActionPreference = "Stop"

    $branch = git branch --show-current

    if (-not $branch.StartsWith("bump")) {
        $n = 1

        while ($true) {
            $newBranch = "bump-$n"
            git show-ref --verify --quiet "refs/heads/$newBranch"
            if ($LASTEXITCODE -ne 0) {
                break
            }
            $n++
        }

        git switch -c $newBranch
        Write-Host "Switched to $newBranch"
    }

    actions-up --yes --min-age 0
    git add .github

    git diff --cached --quiet
    if ($LASTEXITCODE -eq 0) {
        Write-Host "skipping commit"
    } else {
        git commit -m "bump GitHub Actions pinned SHAs"
    }

    uv run scripts/bump_python_deps.py
    git add pyproject.toml

    git diff --cached --quiet
    if ($LASTEXITCODE -eq 0) {
        Write-Host "skipping commit"
    } else {
        git commit -m "bump python dependency-groups"
    }

    cargo upgrade --dry-run --verbose --pinned --verbose
