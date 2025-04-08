from pathlib import Path

import fornax
import imageio.v3 as iio

root = Path(__file__).parents[3]
temp_dir = root / "temp" / "py"
temp_dir.mkdir(parents=True, exist_ok=True)
img_dir = root / "external" / "raw-images" / "images"


def test_libraw_dcraw():
    f = img_dir / "colorchart-5D2-6000K.dng"
    img = fornax.Fornax(f, fornax.decoder.Libraw(), fornax.post_processor.DCRawParams()).process()
    iio.imwrite(temp_dir / "libraw_dcraw.tiff", img)
