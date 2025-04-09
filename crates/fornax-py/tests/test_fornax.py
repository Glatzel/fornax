from pathlib import Path

import fornax
import imageio.v3 as iio  # type: ignore

root = Path(__file__).parents[3]
temp_dir = root / "temp" / "py"
temp_dir.mkdir(parents=True, exist_ok=True)
img_dir = root / "external" / "raw-images" / "images"


def test_dcraw():
    f = img_dir / "colorchart-5D2-6000K.dng"
    img = fornax.Fornax(f, fornax.decoder.Libraw(), fornax.post_processor.DCRawParams()).process()
    iio.imwrite(temp_dir / "libraw_dcraw.tiff", img)


def test_dnc():
    f = img_dir / "colorchart-5D2-6000K.dng"
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
    img = fornax.Fornax(f, dnc, fornax.post_processor.DCRawParams()).process()
    iio.imwrite(temp_dir / "dnc.tiff", img)
    assert False
