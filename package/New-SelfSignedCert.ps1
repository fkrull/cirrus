$ErrorActionPreference = $Stop;

$thumbprint = (New-SelfSignedCertificate `
    -Type Custom `
    -Subject "CN=Cirrus, O=CirrusBackup" `
    -KeyUsage DigitalSignature `
    -FriendlyName "Cirrus" `
    -CertStoreLocation "Cert:\CurrentUser\My" `
    -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
).Thumbprint
Write-Output "Enter pfx file password:"
$password = Read-Host -AsSecureString
Export-PfxCertificate -cert "Cert:\CurrentUser\My\$thumbprint" -FilePath target\certificate.pfx -Password $password
Export-Certificate -cert "Cert:\CurrentUser\My\$thumbprint" -FilePath target\certificate-public.crt
