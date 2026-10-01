# The clean-machine check for the Windows setup file. It runs inside a fresh Windows 11
# (Windows Sandbox, started by run-sandbox.ps1, or any clean virtual machine) and needs only
# Windows PowerShell 5.1.
#
#   powershell -ExecutionPolicy Bypass -File smoke.ps1 -Setup <setup.exe> -Evidence <folder>
#
# It installs silently, starts the Start-menu shortcut, starts a match with no configuration,
# compares the engine's hello version with the installed program's --version, uninstalls,
# and writes results.json (one entry per check, and an overall pass) with screenshots.
# It exits 0 when every check passes and 1 otherwise.

param(
    [Parameter(Mandatory = $true)][string]$Setup,
    [Parameter(Mandatory = $true)][string]$Evidence
)

$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $Evidence | Out-Null
$transcript = Join-Path $Evidence 'smoke.log'
Start-Transcript -Path $transcript -Force | Out-Null

$results = New-Object System.Collections.ArrayList
function Add-Check([string]$Name, [bool]$Pass, [string]$Detail) {
    [void]$results.Add([ordered]@{ name = $Name; pass = $Pass; detail = $Detail })
    $word = if ($Pass) { 'PASS' } else { 'FAIL' }
    Write-Output "[$word] $Name - $Detail"
}
function Add-Fact([string]$Name, [string]$Detail) {
    [void]$results.Add([ordered]@{ name = $Name; pass = $true; fact = $true; detail = $Detail })
    Write-Output "[FACT] $Name - $Detail"
}

$installDir = Join-Path $env:LOCALAPPDATA 'Programs\SoccerManager'
$program = Join-Path $installDir 'engine-cli.exe'
$shortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Soccer Manager.lnk'
$dataDir = Join-Path $env:LOCALAPPDATA 'SoccerManager'
$portFile = Join-Path $dataDir 'engine.port'

function Get-Http([string]$Url) {
    $request = [System.Net.WebRequest]::Create($Url)
    $request.Timeout = 5000
    $response = $request.GetResponse()
    try {
        $reader = New-Object System.IO.StreamReader($response.GetResponseStream())
        return $reader.ReadToEnd()
    } finally {
        $response.Close()
    }
}

function Get-Status([string]$Address) {
    try { return (Get-Http ($Address + 'engine.json')) | ConvertFrom-Json } catch { return $null }
}

function Wait-Running([string]$Address, [int]$Seconds) {
    $deadline = (Get-Date).AddSeconds($Seconds)
    while ((Get-Date) -lt $deadline) {
        $status = Get-Status $Address
        if ($status -and $status.'engine.state' -eq 'running' -and $status.'socket.port') {
            return $status
        }
        Start-Sleep -Milliseconds 250
    }
    return (Get-Status $Address)
}

function Stop-Game {
    Get-Process -Name 'engine-cli' -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 500
}

# Starts `engine-cli launch` from the install folder and returns the process and the address.
function Start-Launcher([string]$Tag) {
    $out = Join-Path $Evidence "launch-$Tag.out.txt"
    $err = Join-Path $Evidence "launch-$Tag.err.txt"
    $process = Start-Process -FilePath $program -ArgumentList @('launch', '--minutes', '1') `
        -WorkingDirectory $installDir -RedirectStandardOutput $out -RedirectStandardError $err `
        -WindowStyle Hidden -PassThru
    $deadline = (Get-Date).AddSeconds(30)
    $address = $null
    while ((Get-Date) -lt $deadline -and -not $address) {
        if (Test-Path $out) {
            $line = Get-Content $out -TotalCount 1 -ErrorAction SilentlyContinue
            if ($line -match '^http://127\.0\.0\.1:\d+/$') { $address = $line }
        }
        if (-not $address) { Start-Sleep -Milliseconds 200 }
    }
    return @{ process = $process; address = $address }
}

function Save-Desktop([string]$Path) {
    Add-Type -AssemblyName System.Windows.Forms, System.Drawing
    $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bitmap = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
    $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
}

# Reads one text message from a socket, joining its fragments.
function Receive-Text($Socket) {
    $buffer = New-Object byte[] 65536
    $segment = New-Object System.ArraySegment[byte] -ArgumentList @(, $buffer)
    $text = New-Object System.Text.StringBuilder
    do {
        $received = $Socket.ReceiveAsync($segment, [System.Threading.CancellationToken]::None).GetAwaiter().GetResult()
        if ($received.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) { return $null }
        if ($received.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Text) {
            [void]$text.Append([System.Text.Encoding]::UTF8.GetString($buffer, 0, $received.Count))
        }
    } while (-not $received.EndOfMessage -or $text.Length -eq 0)
    return $text.ToString()
}

function Send-Text($Socket, [string]$Text) {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    $segment = New-Object System.ArraySegment[byte] -ArgumentList @(, $bytes)
    $Socket.SendAsync($segment, [System.Net.WebSockets.WebSocketMessageType]::Text, $true,
        [System.Threading.CancellationToken]::None).GetAwaiter().GetResult() | Out-Null
}

try {
    # (a) A clean machine may or may not carry the Visual C++ runtime; the game must not care.
    $vcruntime = Test-Path (Join-Path $env:SystemRoot 'System32\vcruntime140.dll')
    Add-Fact 'vcruntime140-present' "vcruntime140.dll in System32: $vcruntime"
    Add-Fact 'data-folder-before' "data folder existed before install: $(Test-Path $dataDir)"

    # (b) Silent install.
    $install = Start-Process -FilePath $Setup -ArgumentList '/S' -Wait -PassThru
    Add-Check 'install-exit-code' ($install.ExitCode -eq 0) "setup /S exited $($install.ExitCode)"

    # (c) The installed files and the shortcut.
    $missing = @()
    foreach ($path in @($program, (Join-Path $installDir 'content\attributes.json'),
            (Join-Path $installDir 'web\index.html'), (Join-Path $installDir 'web\fonts\OFL-Saira.txt'),
            (Join-Path $installDir 'Uninstall.exe'),
            $shortcut)) {
        if (-not (Test-Path $path)) { $missing += $path }
    }
    Add-Check 'installed-files' ($missing.Count -eq 0) ($(if ($missing.Count) { "missing: $($missing -join '; ')" } else { 'program, content, page, font licences, uninstaller, and shortcut present' }))
    $installedVersion = ((& $program --version) -split '\s+')[1]
    Add-Fact 'installed-version' "engine-cli --version: $installedVersion"

    # (d) The Start-menu shortcut: the launcher starts, runs the engine, and opens the page.
    $shortcutStart = Get-Date
    Start-Process -FilePath $shortcut
    $deadline = (Get-Date).AddSeconds(60)
    $portSeen = $false
    while ((Get-Date) -lt $deadline) {
        if ((Test-Path $portFile) -and ((Get-Item $portFile).LastWriteTime -ge $shortcutStart.AddSeconds(-2))) {
            $portSeen = $true
            break
        }
        Start-Sleep -Milliseconds 250
    }
    Add-Check 'shortcut-engine-port' $portSeen "engine.port written after the shortcut started: $portSeen"
    $launcher = Get-CimInstance Win32_Process -Filter "Name = 'engine-cli.exe'" |
        Where-Object { $_.CommandLine -match ' launch' } | Select-Object -First 1
    $shortcutRunning = $false
    $shortcutDetail = 'no launcher process found'
    if ($launcher) {
        $ports = Get-NetTCPConnection -OwningProcess $launcher.ProcessId -State Listen -ErrorAction SilentlyContinue |
            Where-Object { $_.LocalAddress -eq '127.0.0.1' } | Select-Object -ExpandProperty LocalPort
        foreach ($port in $ports) {
            $status = Wait-Running "http://127.0.0.1:$port/" 30
            if ($status -and $status.'engine.state' -eq 'running') {
                $shortcutRunning = $true
                $shortcutDetail = "launcher page on port $port reports engine running on socket $($status.'socket.port')"
                break
            }
        }
        if (-not $shortcutRunning) { $shortcutDetail = "launcher $($launcher.ProcessId) found; no page port reported a running engine" }
    }
    Add-Check 'shortcut-engine-running' $shortcutRunning $shortcutDetail
    # The browser is the machine's own: on a first start it can show its welcome page first,
    # so whether it opened is recorded, and the screenshot shows what the player sees.
    Start-Sleep -Seconds 20
    $browsers = @(Get-Process -Name 'msedge' -ErrorAction SilentlyContinue).Count
    Add-Fact 'shortcut-browser' "browser processes after the shortcut started: $browsers"
    Save-Desktop (Join-Path $Evidence 'desktop.png')
    Stop-Game

    # (e) A launch with no flags but the match length: the page renders at 1280 by 800.
    $second = Start-Launcher 'page'
    Add-Check 'launch-address' ([bool]$second.address) "address printed: $($second.address)"
    if ($second.address) {
        $status = Wait-Running $second.address 60
        Add-Check 'launch-engine-running' ($status -and $status.'engine.state' -eq 'running') "engine.state: $($status.'engine.state')"
        # The page is the built viewer: its module script comes from assets/ as JavaScript.
        $index = Get-Content -Raw (Join-Path $installDir 'web\index.html')
        $moduleType = $null
        $module = $null
        if ($index -match '<script type="module"[^>]*src="/?(assets/[^"]+)"') {
            $module = $Matches[1]
            try {
                $request = [System.Net.WebRequest]::Create($second.address + $module)
                $request.Timeout = 5000
                $response = $request.GetResponse()
                $moduleType = $response.ContentType
                $response.Close()
            } catch { $moduleType = "error: $($_.Exception.Message)" }
        }
        Add-Check 'viewer-module' ([bool]$module -and "$moduleType" -match '^text/javascript') "module '$module' served as '$moduleType'"
        $edge = Join-Path ${env:ProgramFiles(x86)} 'Microsoft\Edge\Application\msedge.exe'
        if (Test-Path $edge) {
            $edgeProfile = Join-Path $env:TEMP 'smoke-edge-profile'
            $page = Join-Path $Evidence 'page.png'
            Start-Process -FilePath $edge -Wait -ArgumentList @('--headless', '--disable-gpu',
                '--no-first-run', "--user-data-dir=$edgeProfile", '--window-size=1280,800',
                '--virtual-time-budget=8000', "--screenshot=$page", $second.address) | Out-Null
            Add-Check 'page-screenshot' (Test-Path $page) "headless page screenshot written: $(Test-Path $page)"
        } else {
            Add-Fact 'page-screenshot' 'Microsoft Edge is not installed; no page screenshot'
        }
    }
    Stop-Game

    # (f) A match starts: read the hello, compare its version, start, and see the kick-off.
    $third = Start-Launcher 'match'
    $helloVersion = $null
    $kickOff = $false
    $matchDetail = 'no address printed'
    if ($third.address) {
        $status = Wait-Running $third.address 60
        if ($status -and $status.'socket.port') {
            $socket = New-Object System.Net.WebSockets.ClientWebSocket
            $socket.Options.SetRequestHeader('Origin', 'http://127.0.0.1')
            $uri = [Uri]("ws://127.0.0.1:$($status.'socket.port')/?v=$($status.'protocol.version')")
            $socket.ConnectAsync($uri, [System.Threading.CancellationToken]::None).GetAwaiter().GetResult()
            $hello = (Receive-Text $socket) | ConvertFrom-Json
            if ($hello.type -eq 'hello') { $helloVersion = $hello.'engine.version' }
            Send-Text $socket '{"type":"start"}'
            $events = Join-Path $dataDir ("matches\" + $status.'match.id' + '\events.jsonl')
            $deadline = (Get-Date).AddSeconds(120)
            while ((Get-Date) -lt $deadline -and -not $kickOff) {
                if (Test-Path $events) {
                    $kickOff = [bool](Select-String -Path $events -SimpleMatch '"event.type":"kick-off"' -Quiet)
                }
                if (-not $kickOff) { Start-Sleep -Milliseconds 500 }
            }
            if ($kickOff) { Copy-Item $events (Join-Path $Evidence 'events.jsonl') }
            $matchDetail = "match $($status.'match.id'); kick-off record in events.jsonl: $kickOff"
            $socket.Dispose()
        } else {
            $matchDetail = "engine never ran; engine.state: $($status.'engine.state')"
        }
    }
    Add-Check 'match-kick-off' $kickOff $matchDetail
    Add-Check 'hello-version' ($helloVersion -and $helloVersion -eq $installedVersion) "hello engine.version '$helloVersion', installed --version '$installedVersion'"
    Stop-Game

    # (g) Silent uninstall removes the program and keeps the matches.
    $uninstaller = Join-Path $installDir 'Uninstall.exe'
    if (Test-Path $uninstaller) {
        Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait | Out-Null
        # The uninstaller copies itself away and returns before its copy finishes.
        $deadline = (Get-Date).AddSeconds(30)
        while ((Get-Date) -lt $deadline -and (Test-Path $installDir)) { Start-Sleep -Milliseconds 250 }
    }
    Add-Check 'uninstall-removes-program' (-not (Test-Path $installDir)) "install folder gone: $(-not (Test-Path $installDir))"
    Add-Check 'uninstall-keeps-matches' (Test-Path (Join-Path $dataDir 'matches')) "matches folder kept: $(Test-Path (Join-Path $dataDir 'matches'))"
} catch {
    Add-Check 'smoke-script' $false "stopped: $($_.Exception.Message)"
    Stop-Game
}

$checks = @($results | Where-Object { -not $_.Contains('fact') })
$pass = ($checks.Count -gt 0) -and (@($checks | Where-Object { -not $_.pass }).Count -eq 0)
$report = [ordered]@{
    platform = 'windows-x64'
    setup = (Split-Path -Leaf $Setup)
    at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    pass = $pass
    checks = $results
}
$report | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 -Path (Join-Path $Evidence 'results.json')
Write-Output "overall: $(if ($pass) { 'PASS' } else { 'FAIL' })"
Stop-Transcript | Out-Null
if ($pass) { exit 0 } else { exit 1 }
