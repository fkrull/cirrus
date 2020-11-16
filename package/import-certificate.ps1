param([string]$certFile)

$ErrorActionPreference = $Stop;

$securePassword = ConvertTo-SecureString -String $env:CERTIFICATE_PASSWORD -Force -AsPlainText
$thumbprint = Import-PfxCertificate -CertStoreLocation Cert:\LocalMachine\My $certFile -Password $securePassword

return $thumbprint.Thumbprint
