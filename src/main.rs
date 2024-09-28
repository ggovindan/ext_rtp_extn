use gst::{ClockTime, DebugGraphDetails, Element};
use gst::prelude::*;
use gst::*;
use lazy_static::lazy_static;
use std::sync::{Arc, Once, Mutex};

lazy_static! {
    static ref previous_pts: Mutex<ClockTime> = Mutex::new(ClockTime::from_seconds(0));
}

fn main() {
    // Initialize GStreamer
    gst::init().unwrap();

    // Create elements for the pipeline
    //let pipeline_str = "videotestsrc ! video/x-raw,width=1280,height=1080,framerate=30/1 ! x264enc ! h264parse name=h264parse ! hlssink2.video hlssink2 name=hlssink2 playlist-length=5 max-files=7 target-duration=2 send-keyframe-requests=true playlist-location=stream.m3u8";

    // Working version!!
    let pipeline_str = "videotestsrc ! video/x-raw,width=1280,height=1080,framerate=30/1 ! x264enc ! h264parse name=h264parse ! splitmuxsink name=splitmuxsink muxer=mpegtsmux max-size-time=2000000000 location=test%05d.ts audiotestsrc is-live=true ! faac name=faac ! splitmuxsink.audio_0";

    // muxing with ts segment works
    //let pipeline_str = "videotestsrc ! video/x-raw,width=1280,height=1080,framerate=30/1 ! x264enc ! h264parse name=h264parse ! mpegtsmux ! filesink location=dip.ts";

    let pipe_elem = gst::parse_launch(pipeline_str).unwrap();

    let pipeline = pipe_elem.clone().downcast::<gst::Pipeline>().unwrap();

    // Get the pipeline objects
    let h264parse = pipeline.by_name("h264parse").unwrap();


    let pay_src_pad = h264parse.static_pad("src").unwrap();
    pay_src_pad.add_probe(gst::PadProbeType::BUFFER, move |_, probe_info| {

        if let Some(probe_data) = probe_info.data.as_mut() {
            if let gst::PadProbeData::Buffer(ref mut buffer) = probe_data {
                let size = buffer.size();
                let pts = buffer.pts().unwrap_or_else(|| ClockTime::from_seconds(0));
                let dts = buffer.dts().unwrap_or_else(|| ClockTime::from_seconds(0));

                // Ensure we print the PTS value
                println!("Original video PTS: {}", pts.seconds());


                // Calculate new PTS, ensure it's monotonically increasing
                let prev_pts = *previous_pts.lock().unwrap();
                let mut new_pts = pts;
                if pts > prev_pts {
                    new_pts = pts + ClockTime::from_seconds(20000)
               }

                // Update DTS in relation to PTS (DTS should always be <= PTS)
                let new_dts = if dts <= pts {
                    dts + ClockTime::from_seconds(19999)
                } else {
                    dts
                };
               //
                println!("New video PTS: {}, New DTS: {}", new_pts.seconds(), new_dts.seconds());

                let buffer_mut = buffer.get_mut().unwrap();
                buffer_mut.set_pts(new_pts);
                buffer_mut.set_dts(new_dts);

            }
        }
        gst::PadProbeReturn::Pass
    });

    let faac = pipeline.by_name("faac").unwrap();

    let faac_src_pad = faac.static_pad("src").unwrap();

    faac_src_pad.add_probe(gst::PadProbeType::BUFFER, move |_, probe_info| {
        if let Some(probe_data) = probe_info.data.as_mut() {
            if let gst::PadProbeData::Buffer(ref mut buffer) = probe_data {
                let size = buffer.size();
                let pts = buffer.pts().unwrap_or_else(|| ClockTime::from_seconds(0));
                let dts = buffer.dts().unwrap_or_else(|| ClockTime::from_seconds(0));

                // Ensure we print the PTS value
                println!("Original audio PTS: {}", pts.seconds());


                // Calculate new PTS, ensure it's monotonically increasing
                let prev_pts = *previous_pts.lock().unwrap();
                let mut new_pts = pts;
                if pts > prev_pts {
                    new_pts = pts + ClockTime::from_seconds(20000)
                }

                // Update DTS in relation to PTS (DTS should always be <= PTS)
                let new_dts = if dts <= pts {
                    dts + ClockTime::from_seconds(19999)
                } else {
                    dts
                };
                //
                println!("New audio PTS: {}, New DTS: {}", new_pts.seconds(), new_dts.seconds());

                let buffer_mut = buffer.get_mut().unwrap();
                buffer_mut.set_pts(new_pts);
                buffer_mut.set_dts(new_dts);

            }
        }
        gst::PadProbeReturn::Pass
    });

    let splitmuxsink = pipeline.by_name("splitmuxsink").unwrap();
    splitmuxsink.connect("split-now", true, |test| {
        println!("got the callback for split-now {:#?}", test);
        None
    });

    // Add RTP header extension
    let bus = pipeline.bus().unwrap();
    let pipeline_weak = pipeline.downgrade();
    let _watch = bus
        .add_watch_local(move |_, msg| {
            if let Some(pipeline) = pipeline_weak.upgrade() {
                match msg.view() {
                    gst::MessageView::Element(msg) => {
                        println!("msg.name()={}", msg.src().unwrap().name());
                    }
                    _ => (),
                }
            }
            glib::ControlFlow::Continue
        })
        .unwrap();

    // Start the pipeline
    pipeline.set_state(gst::State::Playing).unwrap();

    // Run the main loop
    let main_loop = glib::MainLoop::new(None, false);
    main_loop.run();
}