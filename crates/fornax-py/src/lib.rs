use std::path::PathBuf;

use clerk::LogLevel;
use fornax::fornax_dalim::Dalim;
use fornax::libraw::DCRawParams;
use fornax::{dnc, libraw};
use numpy::{PyArray, PyArrayMethods};
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use pyo3::{Python, pyfunction};
use rmp_serde::Deserializer;
use serde::Deserialize;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
enum PyOutputBits {
    Unsigned8,
    Unsigned16,
    Float32,
}
impl From<&str> for PyOutputBits {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "u8" => Self::Unsigned8,
            "u16" => Self::Unsigned16,
            "f32" => Self::Float32,
            bits => panic!("Unknown output bits: {bits}"),
        }
    }
}
enum PyDecoder {
    Libraw,
}
impl From<&str> for PyDecoder {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "libraw" => PyDecoder::Libraw,
            _ => panic!("Unknown decoder."),
        }
    }
}
enum PyPostProcessor {
    Dalim,
    Libraw,
}
impl From<&str> for PyPostProcessor {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "dalim" => PyPostProcessor::Dalim,
            "libraw" => PyPostProcessor::Libraw,
            _ => panic!("Unknown decoder."),
        }
    }
}
macro_rules! raw_workflow {
    ($py:expr,$file:expr,$decoder:expr,$post_processor:expr,$o:ty) => {{
        let manager = fornax::Fornax::new($decoder, $post_processor);
        let img: image::ImageBuffer<image::Rgb<$o>, Vec<$o>> =
            manager.decode_file($file).unwrap().post_process().unwrap();
        let img_array = PyArray::from_slice($py, img.as_ref())
            .reshape([img.height() as usize, img.width() as usize, 3])
            .unwrap();
        (img_array,).into_pyobject($py)
    }};
}

#[allow(clippy::too_many_arguments)]
#[pyfunction]
fn py_process<'a>(
    py: Python<'a>,
    file: PathBuf,
    output_bits: &str,
    decoder: &str,
    _decoder_params: &'a [u8],
    post_processor: &str,
    post_processor_params: &'a [u8],
    dnc_params: Option<&'a [u8]>,
) -> Result<pyo3::Bound<'a, PyTuple>, PyErr> {
    // convert with dnc
    let file = if let Some(params) = dnc_params {
        let dnc = dnc::Dnc::new(Deserialize::deserialize(&mut Deserializer::new(params)).unwrap());
        dnc.convert(&file).unwrap()
    } else {
        file
    };
    let output_bits = PyOutputBits::from(output_bits);
    let decoder = PyDecoder::from(decoder);
    let post_processor = PyPostProcessor::from(post_processor);
    match (decoder, post_processor, output_bits) {
        (PyDecoder::Libraw, PyPostProcessor::Dalim, PyOutputBits::Unsigned8) => {
            let libraw = libraw::Libraw::new(None);
            let dalim_params =
                Deserialize::deserialize(&mut Deserializer::new(post_processor_params)).unwrap();
            let dalim: Dalim<u8> = Dalim::new(dalim_params);
            raw_workflow!(py, &file, libraw, dalim, u8)
        }
        (PyDecoder::Libraw, PyPostProcessor::Dalim, PyOutputBits::Unsigned16) => {
            let libraw = libraw::Libraw::new(None);
            let dalim_params =
                Deserialize::deserialize(&mut Deserializer::new(post_processor_params)).unwrap();
            let dalim: Dalim<u16> = Dalim::new(dalim_params);
            raw_workflow!(py, &file, libraw, dalim, u16)
        }
        (PyDecoder::Libraw, PyPostProcessor::Dalim, PyOutputBits::Float32) => {
            let libraw = libraw::Libraw::new(None);
            let dalim_params =
                Deserialize::deserialize(&mut Deserializer::new(post_processor_params)).unwrap();
            let dalim: Dalim<f32> = Dalim::new(dalim_params);
            raw_workflow!(py, &file, libraw, dalim, f32)
        }
        (PyDecoder::Libraw, PyPostProcessor::Libraw, PyOutputBits::Unsigned8) => {
            let post_processor_params: DCRawParams =
                Deserialize::deserialize(&mut Deserializer::new(post_processor_params)).unwrap();
            let libraw = libraw::Libraw::new(Some(post_processor_params));
            raw_workflow!(py, &file, &libraw, &libraw, u8)
        }
        (PyDecoder::Libraw, PyPostProcessor::Libraw, PyOutputBits::Unsigned16) => {
            let post_processor_params: DCRawParams =
                Deserialize::deserialize(&mut Deserializer::new(post_processor_params)).unwrap();
            let libraw = libraw::Libraw::new(Some(post_processor_params));
            raw_workflow!(py, &file, &libraw, &libraw, u16)
        }
        (PyDecoder::Libraw, PyPostProcessor::Libraw, PyOutputBits::Float32) => {
            let post_processor_params: DCRawParams =
                Deserialize::deserialize(&mut Deserializer::new(post_processor_params)).unwrap();
            let libraw = libraw::Libraw::new(Some(post_processor_params));
            raw_workflow!(py, &file, &libraw, &libraw, f32)
        }
    }
}

#[pyfunction]
pub fn py_init_tracing(level: u8, color: bool) {
    let level = match level {
        1 => LogLevel::ERROR,
        2 => LogLevel::WARN,
        3 => LogLevel::INFO,
        4 => LogLevel::DEBUG,
        5 => LogLevel::TRACE,
        _ => LogLevel::OFF,
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
