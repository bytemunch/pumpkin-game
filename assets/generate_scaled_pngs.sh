FILES="apple
bat
candy_apple
frankenstein
ghost
mummy
pumpkin
skull
spider
sweet
vampire"

SCALES="512
256
128
64
32"

for f in $FILES; do
	if [ -f "$f.png" ]; then
		echo "$f"
		for s in $SCALES; do
			ffmpeg -i "$f.png" -vf scale=$s:$s "$f@$s.png"
		done
	fi
done
