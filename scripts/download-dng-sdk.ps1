aria2c -c -x16 -s16 `
    -d $PSScriptRoot/../temp/download `
    https://www.adobe.com/go/dng_sdk -o "dng-sdk.zip"
7z x "$PSScriptRoot/../temp/download/dng-sdk.zip" "-o$PSScriptRoot/../external"
