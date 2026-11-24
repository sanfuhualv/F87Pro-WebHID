$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Drawing
$source=Join-Path $PSScriptRoot '../Web/favicon.png'
$iconDir=Join-Path $PSScriptRoot 'src-tauri/icons'
New-Item -ItemType Directory -Path $iconDir -Force | Out-Null
$inputImage=[System.Drawing.Bitmap]::new((Resolve-Path $source).Path)
$entries=[System.Collections.Generic.List[object]]::new()
try {
    # Native Windows DIB entries preserve alpha and support resource compilers.
    # Tauri uses the first entry for window/tray rendering, so keep 256 first.
    foreach($size in @(256,128,64,48,32,24,16)) {
        $image=[System.Drawing.Bitmap]::new($size,$size,[System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $graphics=[System.Drawing.Graphics]::FromImage($image)
        try {
            $graphics.Clear([System.Drawing.Color]::Transparent)
            $graphics.CompositingMode=[System.Drawing.Drawing2D.CompositingMode]::SourceCopy
            $graphics.InterpolationMode=[System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.PixelOffsetMode=[System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
            $graphics.DrawImage($inputImage,[System.Drawing.Rectangle]::new(0,0,$size,$size),[System.Drawing.Rectangle]::new(0,0,$inputImage.Width,$inputImage.Height),[System.Drawing.GraphicsUnit]::Pixel)
        } finally { $graphics.Dispose() }
        try {
            $pixels=[byte[]]::new($size*$size*4)
            $maskStride=[int]([Math]::Ceiling($size/32.0)*4)
            $mask=[byte[]]::new($maskStride*$size)
            for($y=0;$y -lt $size;$y++) {for($x=0;$x -lt $size;$x++) {
                $color=$image.GetPixel($x,$y);$row=$size-1-$y;$at=($row*$size+$x)*4
                $pixels[$at]=$color.B;$pixels[$at+1]=$color.G;$pixels[$at+2]=$color.R;$pixels[$at+3]=$color.A
                if($color.A -eq 0){$maskAt=$row*$maskStride+[int][Math]::Floor($x/8.0);$mask[$maskAt]=$mask[$maskAt] -bor (128 -shr ($x%8))}
            }}
            $stream=[System.IO.MemoryStream]::new();$writer=[System.IO.BinaryWriter]::new($stream)
            $writer.Write([uint32]40);$writer.Write([int32]$size);$writer.Write([int32]($size*2))
            $writer.Write([uint16]1);$writer.Write([uint16]32);$writer.Write([uint32]0)
            $writer.Write([uint32]($pixels.Length+$mask.Length))
            1..4 | ForEach-Object {$writer.Write([uint32]0)}
            $writer.Write($pixels);$writer.Write($mask);$writer.Flush()
            $entries.Add([PSCustomObject]@{Size=$size;Bytes=$stream.ToArray()})
            $writer.Dispose();$stream.Dispose()
        } finally { $image.Dispose() }
    }
} finally { $inputImage.Dispose() }
$icoStream=[System.IO.MemoryStream]::new();$icoWriter=[System.IO.BinaryWriter]::new($icoStream)
try {
    $icoWriter.Write([uint16]0);$icoWriter.Write([uint16]1);$icoWriter.Write([uint16]$entries.Count)
    $offset=6+16*$entries.Count
    foreach($entry in $entries){
        $encodedSize=if($entry.Size -eq 256){0}else{$entry.Size}
        $icoWriter.Write([byte]$encodedSize);$icoWriter.Write([byte]$encodedSize);$icoWriter.Write([uint16]0)
        $icoWriter.Write([uint16]1);$icoWriter.Write([uint16]32)
        $icoWriter.Write([uint32]$entry.Bytes.Length);$icoWriter.Write([uint32]$offset)
        $offset+=$entry.Bytes.Length
    }
    foreach($entry in $entries){$icoWriter.Write([byte[]]$entry.Bytes)}
    $icoWriter.Flush();[System.IO.File]::WriteAllBytes((Join-Path $iconDir 'icon.ico'),$icoStream.ToArray())
} finally {$icoWriter.Dispose();$icoStream.Dispose()}
Copy-Item -LiteralPath $source -Destination (Join-Path $iconDir 'icon.png') -Force
Write-Host 'Prepared Windows EXE/window/tray icons from favicon.png.'
