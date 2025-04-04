Set-Location $PSScriptRoot

# use custom triplet
$triplet = Resolve-Path ./triplet

# install static dependency
Write-Output "::group::static"
&./vcpkg/vcpkg.exe install `
    --overlay-triplets=$triplet `
    --triplet x64-windows-static `
    --x-install-root ./installed `
    --vcpkg-root ./vcpkg
Write-Output "::endgroup::"
