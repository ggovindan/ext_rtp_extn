use gst::{DebugGraphDetails, Element};
use gst::prelude::*;
use priority_queue::PriorityQueue;
use gst::*;
use anyhow::{Error, format_err};

use gst_rtp::RTPBuffer;

fn main() {
    // Initialize GStreamer
    gst::init().unwrap();

    // Create elements for the pipeline
    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("videotestsrc").build().unwrap();
    let capsfilter = gst::ElementFactory::make("capsfilter").build().unwrap();
    let enc = gst::ElementFactory::make("x264enc").build().unwrap();
    let h264parse = gst::ElementFactory::make("h264parse").build().unwrap();
    let pay = gst::ElementFactory::make("rtph264pay").build().unwrap();
    let udpsink = ElementFactory::make("udpsink").build().unwrap();

    h264parse.set_property("config-interval", 1);
    pay.set_property("config-interval", 1);
    udpsink.set_property("host", "127.0.0.1");
    udpsink.set_property("port", 56001 as i32);
    //let sink = gst::ElementFactory::make("fakesink").build().unwrap();

    // Set properties
    capsfilter.set_property_from_str("caps", "video/x-raw, width=1280, height=720, framerate=30/1");
    src.set_property_from_str("is-live", "true");

    // Build the pipeline
    pipeline.add_many(&[&src, &capsfilter, &enc, &h264parse, &pay, &udpsink]).unwrap();
    gst::Element::link_many(&[&src, &capsfilter, &enc, &h264parse, &pay, &udpsink]).unwrap();


    let pay_src_pad = pay.static_pad("src").unwrap();
    pay_src_pad.add_probe(gst::PadProbeType::BUFFER, |pad, probe_info| {
        if let Some(probe_data) = probe_info.data.as_mut() {
            if let gst::PadProbeData::Buffer(ref mut buffer) = probe_data {
                let size = buffer.size();
                match buffer.pts() {
                    Some(pts) => {
                        println!("ptstime={}", pts.seconds())
                    },
                    None => {
                        println!("No PTS, cannot get bandwidth")
                    }
                }

                let b = buffer.get_mut().unwrap();
                let mut rtp_buffer = RTPBuffer::from_buffer_writable(b).unwrap();

                    let pts = rtp_buffer.buffer().pts().unwrap();
                    // Convert the PTS to bytes
                    let pts_bytes = pts.to_be_bytes();
                    let extension_data = &pts_bytes[..];

                    let appbits = 5; // Custom application-specific bits
                    let id = 1; // Custom extension ID
                    let result = rtp_buffer.add_extension_onebyte_header(id, extension_data);//.add_extension_twobytes_header(appbits, id, extension_data);

                    if let Err(e) = result {
                        eprintln!("Failed to add RTP header extension: {:?}", e);
                    }

            }
        }
        gst::PadProbeReturn::Ok
    });

    let udpsink_sink_pad = udpsink.static_pad("sink").unwrap();
    udpsink_sink_pad.add_probe(gst::PadProbeType::BUFFER, |pad, probe_info| {
        if let Some(probe_data) = probe_info.data.as_ref() {
            if let gst::PadProbeData::Buffer(buffer) = probe_data {
                let rtp_buffer = RTPBuffer::from_buffer_readable(buffer).unwrap();
                // Check for RTP extension header
                if let Some(extension_data) = rtp_buffer.extension_onebyte_header(1, 0) { //extension_twobytes_header(1, 0) {
                    println!("RTP Extension present:");
                    //println!("App bits: {}", bits);
                    println!("Extension data: {:?}", extension_data);

                    // Convert the extension data back to PTS
                    if extension_data.len() != 0 {
                        //let mut pts_bytes = [0u8; 8];
                        //pts_bytes[..4].copy_from_slice(&extension_data[..4]);  // Copy the first 4 bytes
                        //let pts = u64::from_be_bytes(pts_bytes);
                        let pts = u64::from_be_bytes(extension_data.try_into().unwrap());
                        println!("Extracted PTS from RTP extension: {}", pts);
                    }
                } else {
                    println!("No RTP Extension found");
                }
                match rtp_buffer.buffer().pts() {
                    Some(pts) => {
                        println!("udpsink buffer.pts={}", pts.seconds());
                    },
                    None => {
                        println!("No PTS, cannot get bandwidth");
                    }
                }
            }
        }
        gst::PadProbeReturn::Ok
    }).unwrap();



    // Add RTP header extension
    let bus = pipeline.bus().unwrap();
    let pipeline_weak = pipeline.downgrade();
    let _watch = bus.add_watch_local(move |_, msg| {
        if let Some(pipeline) = pipeline_weak.upgrade() {
            match msg.view() {
                gst::MessageView::Element(msg) => {
                    println!("msg.name()={}", msg.src().unwrap().name());
                },
                _ => (),
            }
        }
        glib::ControlFlow::Continue
    }).unwrap();

    // Start the pipeline
    pipeline.set_state(gst::State::Playing).unwrap();

    // Run the main loop
    let main_loop = glib::MainLoop::new(None, false);
    main_loop.run();
}
