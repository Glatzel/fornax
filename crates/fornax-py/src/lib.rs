use std::path::PathBuf;

use fornax::dnc::Dnc;
use fornax::libraw::Libraw;
use fornax::libraw::dcraw::DCRawParams;
use numpy::{PyArray, PyArrayMethods};
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use pyo3::{Python, pyfunction};
use rmp_serde::Deserializer;
use serde::Deserialize;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
enum PyDecoder {
    Libraw,
}
impl From<&str> for PyDecoder {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "libraw" => PyDecoder::Libraw,
            _ => panic!("Unknow decoder."),
        }
    }
}
enum PyPostPorcessor {
    Libraw,
}
impl From<&str> for PyPostPorcessor {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "libraw" => PyPostPorcessor::Libraw,
            _ => panic!("Unknow decoder."),
        }
    }
}
#[pyfunction]
fn py_process<'a>(
    py: Python<'a>,
    file: PathBuf,
    dnc_params: Option<&'a [u8]>,
    decoder: &str,
    _decoder_params: &'a [u8],
    post_processor: &str,
    post_processor_params: &'a [u8],
) -> Result<pyo3::Bound<'a, PyTuple>, PyErr> {
    let file = if let Some(params) = dnc_params {
        let dnc = Dnc::new(Deserialize::deserialize(&mut Deserializer::new(params)).unwrap());
        dnc.convert(&file).unwrap()
    } else {
        file
    };
    let img = match (
        PyDecoder::from(decoder),
        PyPostPorcessor::from(post_processor),
    ) {
        (PyDecoder::Libraw, PyPostPorcessor::Libraw) => {
            let post_processor_params: DCRawParams =
                Deserialize::deserialize(&mut Deserializer::new(post_processor_params)).unwrap();
            let libraw = Libraw::new(Some(post_processor_params));

            let mut manager = fornax::Fornax::new(&libraw, &libraw);
            manager.decode_file(&file).unwrap().post_process().unwrap()
        }
    };

    match img {
        fornax::FornaxProcessedImage::Null => panic!("Process failed."),
        fornax::FornaxProcessedImage::Mono8(img) => {
            let img_array = PyArray::from_slice(py, img.as_ref());
            let img_array = img_array
                .reshape([img.height() as usize, img.width() as usize, 1])
                .unwrap();
            (img_array,).into_pyobject(py)
        }
        fornax::FornaxProcessedImage::Mono16(img) => {
            let img_array = PyArray::from_slice(py, img.as_ref());
            let img_array = img_array
                .reshape([img.height() as usize, img.width() as usize, 1])
                .unwrap();
            (img_array,).into_pyobject(py)
        }
        fornax::FornaxProcessedImage::MonoF32(img) => {
            let img_array = PyArray::from_slice(py, img.as_ref());
            let img_array = img_array
                .reshape([img.height() as usize, img.width() as usize, 1])
                .unwrap();
            (img_array,).into_pyobject(py)
        }
        fornax::FornaxProcessedImage::Rgb8(img) => {
            let img_array = PyArray::from_slice(py, img.as_ref());
            let img_array = img_array
                .reshape([img.height() as usize, img.width() as usize, 3])
                .unwrap();
            (img_array,).into_pyobject(py)
        }
        fornax::FornaxProcessedImage::Rgb16(img) => {
            let img_array = PyArray::from_slice(py, img.as_ref());
            let img_array = img_array
                .reshape([img.height() as usize, img.width() as usize, 3])
                .unwrap();
            (img_array,).into_pyobject(py)
        }
        fornax::FornaxProcessedImage::RgbF32(img) => {
            let img_array = PyArray::from_slice(py, img.as_ref());
            let img_array = img_array
                .reshape([img.height() as usize, img.width() as usize, 3])
                .unwrap();
            (img_array,).into_pyobject(py)
        }
    }
}

#[pyfunction]
pub fn py_init_tracing(level: u8, color: bool) {
    let level = match level {
        1 => LevelFilter::ERROR,
        2 => LevelFilter::WARN,
        3 => LevelFilter::INFO,
        4 => LevelFilter::DEBUG,
        5 => LevelFilter::TRACE,
        _ => LevelFilter::OFF,
    };
    tracing_subscriber::registry()
        .with(clerk::terminal_layer(level, color))
        .init();
}
#[pymodule]
fn fornax_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(py_process))?;
    m.add_wrapped(wrap_pyfunction!(py_init_tracing))?;
    Ok(())
}
