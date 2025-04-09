from pathlib import Path

import fornax
import imageio.v3 as iio  # type: ignore

root = Path(__file__).parents[3]
temp_dir = root / "temp" / "py"
temp_dir.mkdir(parents=True, exist_ok=True)
img_dir = root / "external" / "raw-images" / "images"


def test_dcraw():
    f = img_dir / "colorchart-5D2-6000K.dng"
    img = fornax.Fornax(fornax.decoder.Libraw(), fornax.post_processor.DCRawParams()).process(f)
    out_file = temp_dir / "libraw_dcraw.tiff"
    iio.imwrite(out_file, img)
    assert out_file.is_file()


def test_dnc():
    f = img_dir / "colorchart-eos-7d.cr2"
    dnc = fornax.decoder.DncParams(
        compressed=True,
        linear=False,
        embed=False,
        preview=fornax.decoder.DncPreview._None,
        fast_load=False,
        side=1000,
        count=None,
        compatibility=fornax.decoder.DncCompatibility.CR14_0,
        directory=temp_dir / "dnc",
        filename="test_dnc.dng",
        overwrite=True,
    )
    img = fornax.Fornax(dnc, fornax.post_processor.DCRawParams()).process(f)
    out_file = temp_dir / "dnc.tiff"
    iio.imwrite(out_file, img)
    assert out_file.is_file()
