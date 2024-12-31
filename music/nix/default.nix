{
  config,
  pkgs,
}:
let
  bundles = [
    { key = "beatport_tracks_2023-11 (1).zip"; outputHash = "sha256-cyKYjMVeM0oPxMP+JFlVVDsr6KZPuU/sIpjk+WQXxGc="; }
    { key = "beatport_tracks_2023-11.zip"; outputHash = "sha256-u2ufc0zWi0uWqPGuiqVP7XzpuKJoNfJdf6aH79hPX6g="; }
    { key = "beatport_tracks_2023-12 (1).zip"; outputHash = "sha256-ea8OJ9FZn2ccaXq48O+FL0IzWnd+ZeWCatVS4gcxiqw="; }
    { key = "beatport_tracks_2023-12 (2).zip"; outputHash = "sha256-nPWtoiNWfzK1E7zNz3yIfIuIYHnCy4K4y5X2qlp12L4="; }
    { key = "beatport_tracks_2023-12.zip"; outputHash = "sha256-rF1lreMOFzBE++qeLZIg9kFEsqumn7nnfNOMF0WpAe0="; }
    { key = "beatport_tracks_2024-01.zip"; outputHash = "sha256-mQcls6wKg60sT0IpWhXNkqUoPsZ3iQVuCi/JUG8/Af8="; }
    { key = "beatport_tracks_2024-05.zip"; outputHash = "sha256-ewOKG0bi1uDi6ihQOMdu5mPG8i9ylk1glF8h3WBYs5Y="; }
    { key = "beatport_tracks_2024-07.zip"; outputHash = "sha256-O18ehvRmS8qtfiGrWgSuomxY3wV5XCODMncL43j/THo="; }
    { key = "beatport_tracks_2024-08.zip"; outputHash = "sha256-ehV/Knv7nTNkTr/iv7EamWb5YZJUbnlJxfZajqoGNLY="; }
    { key = "beatport_tracks_2024-10-2.zip"; outputHash = "sha256-01rlwZGxsq5RnOjol7Dh45LnUPTN4PuJJXIl8PMHeWw="; }
    { key = "beatport_tracks_2024-10.zip"; outputHash = "sha256-ic+Egn0ecBJwBZiKbmg52u4vyDB3u1EZdG5rvGIhHeY="; }
  ];
  drvs = map (bundle: pkgs.stdenv.mkDerivation {
    name = "bundle";
    src = pkgs.fetchs3 {
      s3url = ''"s3://${(builtins.fromJSON (builtins.getEnv "AWS_ORG")).accounts.musicStorage}-eu-west-1-music/${bundle.key}"'';
      name = pkgs.lib.strings.sanitizeDerivationName bundle.key;
      credentials = {
        access_key_id = builtins.getEnv "AWS_ACCESS_KEY_ID";
        secret_access_key = builtins.getEnv "AWS_SECRET_ACCESS_KEY";
        session_token = builtins.getEnv "AWS_SESSION_TOKEN";
      };
      outputHashAlgo = "sha256";
      region = "eu-west-1";
      inherit (bundle) outputHash;
    };

    dontUnpack = true;

    # Figure out how we can set BPM_LIMIT in a smart way similar to source filtering
    buildPhase = ''
      mkdir -p $out

      mkdir -p files
      cd files

      ${pkgs.unzip}/bin/unzip $src

      BPM_LIMIT=180
      for file in $(dir); do
        (
          chmod +r "$file"
          BPM=$(${pkgs.ffmpeg}/bin/ffmpeg -vn -i "$file" -ar 44100 -ac 1 -f f32le pipe:1 2>/dev/null | ${pkgs.bpm-tools}/bin/bpm -x $BPM_LIMIT -f "%03.0f")
          KEY=$(${pkgs.keyfinder-cli}/bin/keyfinder-cli -n camelot $file);
          mv "$file" "$out/''${BPM}_''${KEY}_''${file}"
        ) &
      done

      wait
    '';
  }) bundles;
in pkgs.stdenv.mkDerivation {
  name = "music-formats";
  src = pkgs.symlinkJoin {
    name = "music";
    paths = drvs;
  };

  outputs = ["out" "flac"];

  dontUnpack = true;

  buildPhase = ''
    mkdir -p $out $flac

    for file in $src/*; do
        (
          ${pkgs.flac}/bin/flac -o $flac/$(basename $file .wav).flac $file
          ln -s $file $out/$file
        ) &
    done

    wait
  '';
}
