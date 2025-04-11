use std::path::PathBuf;

use fornax::dnc::{Dnc, DncParams};
use fornax::libraw::dcraw::DCRawParams;
use fornax::libraw::{DCRaw, Libraw};
use numpy::PyArray;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use pyo3::{Python, pyfunction};
use rmp_serde::Deserializer;
use serde::Deserialize;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload::{self, Handle};
use tracing_subscriber::util::SubscriberInitExt;
enum PyDecoder {
    Libraw,
    Dnc,
}
impl From<&str> for PyDecoder {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "libraw" => PyDecoder::Libraw,
            "dnc" => PyDecoder::Dnc,
            _ => panic!("Unknow decoder."),
        }
    }
}
enum PyPostPorcessor {
    DCRaw,
}
impl From<&str> for PyPostPorcessor {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "dcraw" => PyPostPorcessor::DCRaw,
            _ => panic!("Unknow decoder."),
        }
    }
}
#[pyfunction]
fn py_process<'a>(
    py: Python<'a>,
    file: PathBuf,
    decoder: &str,
    decoder_params: &'a [u8],
    post_processor: &str,
    post_processor_params: &'a [u8],
) -> Result<pyo3::Bound<'a, PyTuple>, PyErr> {
    let img = match (
        PyDecoder::from(decoder),
        PyPostPorcessor::from(post_processor),
    ) {
        (PyDecoder::Libraw, PyPostPorcessor::DCRaw) => {
            let decoder = Libraw::new();
            let post_processor_params: DCRawParams =
                Deserialize::deserialize(&mut Deserializer::new(post_processor_params)).unwrap();
            let mut manager = fornax::Fornax::new(decoder, DCRaw::new(post_processor_params));
            manager.decode_file(&file).unwrap().post_process().unwrap()
        }
        (PyDecoder::Dnc, PyPostPorcessor::DCRaw) => {
            let decoder_params: DncParams =
                Deserialize::deserialize(&mut Deserializer::new(decoder_params)).unwrap();
            let post_processor_params: DCRawParams =
                Deserialize::deserialize(&mut Deserializer::new(post_processor_params)).unwrap();
            let mut manager =
                fornax::Fornax::new(Dnc::new(decoder_params), DCRaw::new(post_processor_params));
            manager.decode_file(&file).unwrap().post_process().unwrap()
        }
    };
    match img {
        fornax::FornaxProcessedImage::None => panic!("Process failed."),
        fornax::FornaxProcessedImage::Mono8(img) => (
            PyArray::from_slice(py, img.as_ref()),
            img.width(),
            img.height(),
            1,
            8,
        )
            .into_pyobject(py),
        fornax::FornaxProcessedImage::Mono16(img) => (
            PyArray::from_slice(py, img.as_ref()),
            img.width(),
            img.height(),
            1,
            16,
        )
            .into_pyobject(py),
        fornax::FornaxProcessedImage::Rgb8(img) => (
            PyArray::from_slice(py, img.as_ref()),
            img.width(),
            img.height(),
            3,
            8,
        )
            .into_pyobject(py),
        fornax::FornaxProcessedImage::Rgb16(img) => (
            PyArray::from_slice(py, img.as_ref()),
            img.width(),
            img.height(),
            3,
            16,
        )
            .into_pyobject(py),
    }
}
static RELOAD_HANDLE: std::sync::LazyLock<Handle<LevelFilter, Registry>> =
    std::sync::LazyLock::new(|| {
        let filter = LevelFilter::OFF;
        let (reload_layer, reload_handle) = reload::Layer::new(filter);
        tracing_subscriber::registry()
            .with(reload_layer)
            .with(clerk::terminal_layer())
            .init();
        reload_handle
    });
#[pyfunction]
pub fn py_set_log_level(level: u8) {
    let level = match level {
        1 => LevelFilter::ERROR,
        2 => LevelFilter::WARN,
        3 => LevelFilter::INFO,
        4 => LevelFilter::DEBUG,
        5 => LevelFilter::TRACE,
        _ => LevelFilter::OFF,
    };
    RELOAD_HANDLE.modify(|filter| *filter = level).unwrap();
}
#[pymodule]
fn fornax_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(py_process))?;
    m.add_wrapped(wrap_pyfunction!(py_set_log_level))?;
    Ok(())
}
