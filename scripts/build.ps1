param([switch]$Test, [string]$Distro = 'AgentOS-Dev')
$projectRoot = Split-Path -Parent $PSScriptRoot
if ($Test) { wsl -d $Distro --cd $projectRoot --exec bash scripts/build.sh test }
else { wsl -d $Distro --cd $projectRoot --exec bash scripts/build.sh }
exit $LASTEXITCODE
