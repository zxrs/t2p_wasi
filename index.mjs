import {init, WASI} from "@wasmer/wasi";

const generate_pdf = async (tiff) => {
    await init();

    let wasi = new WASI({
        env: {},
        args: [],
    });

    const module = await WebAssembly.compileStreaming(fetch("t2p_wasi.wasm"));

    wasi.instantiate(module, {});

    let file = wasi.fs.open("/input", {read: true, write: true, create: true});
    file.write(tiff);
    file.seek(0);

    const exitCode = wasi.start();
    const stderr = wasi.getStderrString();
    console.log(`${stderr} (exit code: ${exitCode})`);

    wasi.fs.removeFile("/input");

    const out = wasi.fs.open("/output", {read: true, write: false, create: false});
    const pdf = out.read();

    wasi.fs.removeFile("/output");

    return pdf;
};

export default generate_pdf;
