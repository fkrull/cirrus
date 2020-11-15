param([string]$thumbprint, [string]$target)

$ErrorActionPreference = $Stop;

Export-Certificate -Type CERT -Cert cert:\CurrentUser\My\$thumbprint -FilePath $target
