for pattern in \
    '*sony*ILCE-7*14bit uncompressed (3:2).ARW' \
    '*canon*eos r*3:2.CR3' \
    '*nikon*Z*Lossless*(3:2).NEF'
do
    pixi run -e data get-raw-samples --after 2020-01-01 "$pattern"
done