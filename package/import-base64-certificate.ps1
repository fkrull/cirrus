param(
    [string]$certificateFileEnv,
    [string]$certificatePasswordEnv,
    [string]$certStore = "Cert:\LocalMachine\My"
)

$ErrorActionPreference = $Stop;

$certFile = New-TemporaryFile
try {
    $certificateBase64 = Get-Content Env:\$certificateFileEnv
    $certificatePassword = Get-Content Env:\$certificatePasswordEnv

    [System.Convert]::FromBase64String($certificateBase64) | Set-Content -Encoding Byte -Path $certFile
    $securePassword = ConvertTo-SecureString -String $certificatePassword -Force -AsPlainText
    $thumbprint = Import-PfxCertificate -CertStoreLocation $certStore $certFile -Password $securePassword
    return $thumbprint.Thumbprint
} finally {
    Delete-Item $certFile
}
