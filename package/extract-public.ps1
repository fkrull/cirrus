param([string]$thumbprint, [string]$target)

$ErrorActionPreference = $Stop;

Export-Certificate -Type CERT -Cert Cert:\LocalMachine\My\$thumbprint -FilePath $target
