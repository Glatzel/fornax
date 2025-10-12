import sys
from pathlib import Path

import fornax
import imageio.v3 as iio  # type: ignore
import pytest

root = Path(__file__).parents[3]
temp_dir = root / "temp" / "fornax-py"
temp_dir.mkdir(parents=True, exist_ok=True)
img_dir = root / "external" / "raw-images" / "images"

fornax.init_tracing(fornax.LogLevel.DEBUG, True)


@pytest.mark.skipif(
    sys.platform not in ("win32", "darwin"),
    reason="Adobe DNC Converter only available on Windows or MacOS.",
)
def test_dnc():
    f = img_dir / "colorchart-eos-7d.cr2"
    dnc = fornax.dnc.DncParams(
        compressed=True,
        linear=False,
        embed=False,
        preview=fornax.dnc.DncPreview._None,
        fast_load=False,
        side=1000,
        count=None,
        compatibility=fornax.dnc.DncCompatibility.CR14_0,
        directory=temp_dir / "dnc",
        filename="test_dnc.dng",
        overwrite=True,
    )
    img = fornax.Fornax(
        output_bits=fornax.FornaxOutputBits.u8,
        dnc_params=dnc,
        decoder_params=fornax.decoder.LibrawParams(),
        post_processor_params=fornax.post_processor.DCRawParams(),
    ).process(f)
    out_file = temp_dir / "test_dnc.tiff"
    iio.imwrite(out_file, img)
    assert out_file.is_file()


def test_libraw_libraw_default():
    f = img_dir / "colorchart-eos-7d.cr2"
    img = fornax.Fornax(
        output_bits=fornax.FornaxOutputBits.u8,
        decoder_params=fornax.decoder.LibrawParams(),
        post_processor_params=fornax.post_processor.DCRawParams(),
    ).process(f)
    out_file = temp_dir / "test_libraw_libraw_default.tiff"
    assert img.shape == (3464, 5202, 3)
    iio.imwrite(out_file, img)
    assert out_file.is_file()


def test_libraw_libraw_custom():
    f = img_dir / "colorchart-eos-7d.cr2"
    params = fornax.post_processor.DCRawParams(
        greybox=None,
        cropbox=None,
        aber=None,
        gamm=(1.0, 1.0),
        user_mul=(0.9, 0.8, 0.7, 0.6),
        bright=0.9,
        threshold=0.1,
        half_size=True,
        four_color_rgb=None,
        highlight=fornax.post_processor.DCRawHighlightMode.Reconstruct4,
        use_auto_wb=True,
        use_camera_wb=True,
        use_camera_matrix=None,
        output_color=fornax.post_processor.DCRawOutputColor.ACES,
        output_profile=None,
        camera_profile=None,
        bad_pixels=None,
        dark_frame=None,
        output_bps=fornax.post_processor.DCRawOutputBps._16bit,
        output_tiff=None,
        user_flip=None,
        user_qual=fornax.post_processor.DCRawUserQual.ModifiedAHD,
        user_black=None,
        user_cblack=None,
        user_sat=None,
        med_passes=3,
        no_auto_bright=True,
        auto_bright_thr=None,
        adjust_maximum_thr=0.001,
        use_fuji_rotate=None,
        green_matching=None,
        dcb_iterations=None,
        dcb_enhance_fl=None,
        fbdd_noiserd=None,
        exp_correct=None,
        exp_shift=None,
        exp_preser=None,
        use_rawspeed=None,
        no_auto_scale=True,
        no_interpolation=True,
    )  # type: ignore
    print(params.model_dump_json())
    img = fornax.Fornax(
        output_bits=fornax.FornaxOutputBits.u16,
        decoder_params=fornax.decoder.LibrawParams(),
        post_processor_params=params,
    ).process(f)
    out_file = temp_dir / "test_libraw_libraw_custom.tiff"
    iio.imwrite(out_file, img)
    assert out_file.is_file()
    # assert False


def test_libraw_dalim():
    f = img_dir / "colorchart-eos-7d.cr2"
    img = fornax.Fornax(
        output_bits=fornax.FornaxOutputBits.u16,
        decoder_params=fornax.decoder.LibrawParams(),
        post_processor_params=fornax.post_processor.DalimParams(
            demosaicer=fornax.post_processor.DalimDemosaicer.Linear
        ),
    ).process(f)
    out_file = temp_dir / "test_libraw_dalim.tiff"
    assert img.shape == (3464, 5202, 3)
    iio.imwrite(out_file, img)
    assert out_file.is_file()
