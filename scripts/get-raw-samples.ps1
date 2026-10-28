param(
    [string[]]$Name = '*',        # wildcard on the nice filename, e.g. 'Canon - EOS 5D*', '*.CR2'
    [string]$OutDir = "$PSScriptRoot/../external/raw.pixls.us",
    [int]$Jobs = 16,
    [switch]$ListOnly
)

$api = 'https://raw.pixls.us/json/getrepository.php?set=all'

# one request, jq pulls the href out of the anchor in column 7
$urls = curl.exe -fsSL $api |
    jq -r '.data[][7] | capture("href=[\u0027\"](?<u>[^\u0027\"]+)[\u0027\"]").u' |
    Sort-Object -Unique

function Safe([string]$s)
{ ($s -replace '[<>:"/\\|?*]', '_').Trim(' .') 
}

$entries = foreach ($u in $urls)
{
    $file = [uri]::UnescapeDataString(($u -split '/')[-1])
    if (-not ($Name | Where-Object { $file -like $_ }))
    { continue 
    }

    $id   = ($u -split '/')[-3]                      # .../getfile.php/<id>/nice/<name>
    $stem = [IO.Path]::GetFileNameWithoutExtension($file)
    $ext  = [IO.Path]::GetExtension($file)

    # "<Make> - <Model> - <rest>"
    if ($stem -match '^(?<make>.+?) - (?<model>.+?) - (?<rest>.+)$')
    {
        $dir  = "$(Safe $Matches.make)/$(Safe $Matches.model)"
        $leaf = Safe "$($Matches.rest) [$id]$ext"
    } else
    {
        $dir  = '_unsorted'
        $leaf = Safe "$stem [$id]$ext"
    }

    [pscustomobject]@{
        Path = "$dir/$leaf"
        Url  = $u -replace ' ', '%20'
    }
}
$entries = @($entries | Sort-Object Path)

if (-not $entries)
{ Write-Warning 'No files matched.'; return
}
if ($ListOnly)
{ $entries | Format-Table -Wrap; "$($entries.Count) files"; return
}

New-Item -ItemType Directory -Force $OutDir | Out-Null
$list = Join-Path $OutDir 'aria2-input.txt'
$entries | ForEach-Object { $_.Url; "  out=$($_.Path)" } | Set-Content -Encoding utf8 $list

aria2c -i $list -d $OutDir -j $Jobs -x 1 -s 1 -c --auto-file-renaming=false --file-allocation=none
