$o = 'C:\d\out'
Start-Process C:\d\in\SoccerManager-0.1.0-windows-x64-setup.exe -ArgumentList '/S' -Wait
$dir = Join-Path $env:LOCALAPPDATA 'Programs\SoccerManager'
$p = Start-Process (Join-Path $dir 'engine-cli.exe') -ArgumentList @('launch','--minutes','1','--open') -WorkingDirectory $dir -RedirectStandardOutput "$o\out.txt" -RedirectStandardError "$o\err.txt" -WindowStyle Hidden -PassThru
Start-Sleep 25
"msedge after launch --open: $(@(Get-Process msedge -ErrorAction SilentlyContinue).Count)" | Out-File "$o\diag.txt"
$assoc = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\Shell\Associations\UrlAssociations\http\UserChoice' -ErrorAction SilentlyContinue
"http UserChoice: $($assoc.ProgId)" | Out-File -Append "$o\diag.txt"
cmd /c assoc .html 2>&1 | Out-File -Append "$o\diag.txt"
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $bm = New-Object System.Drawing.Bitmap($b.Width,$b.Height); $g=[System.Drawing.Graphics]::FromImage($bm); $g.CopyFromScreen($b.Location,[System.Drawing.Point]::Empty,$b.Size); $bm.Save("$o\desk1.png"); 
Start-Process 'http://127.0.0.1:9/' -ErrorAction SilentlyContinue
Start-Sleep 20
"msedge after Start-Process url: $(@(Get-Process msedge -ErrorAction SilentlyContinue).Count)" | Out-File -Append "$o\diag.txt"
$g.CopyFromScreen($b.Location,[System.Drawing.Point]::Empty,$b.Size); $bm.Save("$o\desk2.png")
'done' | Out-File "$o\done.txt"
