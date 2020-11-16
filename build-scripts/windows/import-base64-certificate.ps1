param(
    [string]$certStore = "Cert:\CurrentUser\My"
)

$ErrorActionPreference = $Stop;

$certFile = New-TemporaryFile
try {
    [System.Convert]::FromBase64String($env:APPX_CERTIFICATE_FILE) | Set-Content -Encoding Byte -Path $certFile
    $securePassword = ConvertTo-SecureString -String $env:APPX_CERTIFICATE_PASSWORD -Force -AsPlainText
    $thumbprint = Import-PfxCertificate -CertStoreLocation $certStore $certFile -Password $securePassword
    return $thumbprint.Thumbprint
} finally {
    Remove-Item $certFile
}
