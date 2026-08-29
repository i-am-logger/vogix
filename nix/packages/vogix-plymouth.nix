# The vogix plymouth theme: a script-plugin theme rendered ENTIRELY from
# text — no bitmaps to recolor, so the palette reaches the boot splash by
# construction (the boot→greeter→session color continuity's first link).
# Build-time like console.colors: a runtime `vogix theme set` changes it at
# the next rebuild.
{ lib
, runCommand
, colors ? { }
}:

let
  inherit (import ../modules/lib/color.nix { inherit lib; }) unitRgb;

  # Neutral dark fallback when no vogix user's palette reached the caller.
  slot = name: fallback: colors.${name} or fallback;
  bg = unitRgb (slot "background" "#181818");
  bgSurface = unitRgb (slot "background_surface" "#202020");
  fg = unitRgb (slot "foreground_text" "#d0d0d0");
  muted = unitRgb (slot "foreground_comment" "#707070");
  accent = unitRgb (slot "active" "#5a9aaa");

  f = toString;

  script = ''
    // vogix boot splash — text-only script theme, palette from the theme
    // system. Background gradient base00→base01; wordmark base05; boot
    // progress as accent dots; messages + password prompt in the palette.
    Window.SetBackgroundTopColor(${f bg.r}, ${f bg.g}, ${f bg.b});
    Window.SetBackgroundBottomColor(${f bgSurface.r}, ${f bgSurface.g}, ${f bgSurface.b});

    logo.image = Image.Text("vogix", ${f fg.r}, ${f fg.g}, ${f fg.b}, 1, "Sans 28");
    logo.sprite = Sprite(logo.image);
    logo.sprite.SetPosition(
      Window.GetX() + Window.GetWidth() / 2 - logo.image.GetWidth() / 2,
      Window.GetY() + Window.GetHeight() / 2 - logo.image.GetHeight() / 2,
      100);

    // Boot progress: five dots filling in the accent color.
    dots.count = 5;
    for (i = 0; i < dots.count; i++) {
      dots.off[i] = Image.Text("·", ${f muted.r}, ${f muted.g}, ${f muted.b}, 1, "Sans 22");
      dots.on[i] = Image.Text("•", ${f accent.r}, ${f accent.g}, ${f accent.b}, 1, "Sans 22");
      dots.sprite[i] = Sprite(dots.off[i]);
      dots.sprite[i].SetPosition(
        Window.GetX() + Window.GetWidth() / 2 + (i - dots.count / 2) * 18,
        Window.GetY() + Window.GetHeight() / 2 + logo.image.GetHeight(),
        100);
    }

    fun progress_callback(duration, progress) {
      for (i = 0; i < dots.count; i++) {
        if (progress * dots.count >= i + 1)
          dots.sprite[i].SetImage(dots.on[i]);
        else
          dots.sprite[i].SetImage(dots.off[i]);
      }
    }
    Plymouth.SetBootProgressFunction(progress_callback);

    message.sprite = Sprite();
    fun message_callback(text) {
      message.image = Image.Text(text, ${f muted.r}, ${f muted.g}, ${f muted.b}, 1, "Sans 12");
      message.sprite.SetImage(message.image);
      message.sprite.SetPosition(
        Window.GetX() + Window.GetWidth() / 2 - message.image.GetWidth() / 2,
        Window.GetY() + Window.GetHeight() * 0.8,
        100);
    }
    Plymouth.SetMessageFunction(message_callback);

    // Disk passphrase prompt (LUKS): prompt text + one bullet per typed
    // character, all in the palette.
    prompt.sprite = Sprite();
    bullets.sprite = Sprite();
    fun password_callback(text, bullet_count) {
      prompt_text = text;
      if (prompt_text == "")
        prompt_text = "Passphrase";
      prompt.image = Image.Text(prompt_text, ${f fg.r}, ${f fg.g}, ${f fg.b}, 1, "Sans 14");
      prompt.sprite.SetImage(prompt.image);
      prompt.sprite.SetPosition(
        Window.GetX() + Window.GetWidth() / 2 - prompt.image.GetWidth() / 2,
        Window.GetY() + Window.GetHeight() * 0.66,
        100);
      dots_text = "";
      for (i = 0; i < bullet_count; i++)
        dots_text = dots_text + "•";
      bullets.image = Image.Text(dots_text, ${f accent.r}, ${f accent.g}, ${f accent.b}, 1, "Sans 14");
      bullets.sprite.SetImage(bullets.image);
      bullets.sprite.SetPosition(
        Window.GetX() + Window.GetWidth() / 2 - bullets.image.GetWidth() / 2,
        Window.GetY() + Window.GetHeight() * 0.66 + prompt.image.GetHeight() + 6,
        100);
    }
    Plymouth.SetDisplayPasswordFunction(password_callback);

    fun display_normal_callback() {
      prompt.sprite.SetImage(Image.Text("", 0, 0, 0));
      bullets.sprite.SetImage(Image.Text("", 0, 0, 0));
    }
    Plymouth.SetDisplayNormalFunction(display_normal_callback);
  '';

  themeIni = ''
    [Plymouth Theme]
    Name=Vogix
    Description=Vogix boot splash (palette from the active theme)
    ModuleName=script

    [script]
    ImageDir=/etc/vogix-plymouth
    ScriptFile=/etc/vogix-plymouth/vogix.script
  '';
in
runCommand "vogix-plymouth"
{
  inherit script themeIni;
  passAsFile = [ "script" "themeIni" ];
  meta = {
    description = "Vogix-themed plymouth boot splash";
    license = lib.licenses.cc-by-nc-sa-40;
  };
} ''
  t=$out/share/plymouth/themes/vogix
  mkdir -p $t
  install -m644 $scriptPath $t/vogix.script
  # ScriptFile/ImageDir must be the FINAL store path, only known now:
  substitute $themeIniPath $t/vogix.plymouth \
    --replace-fail /etc/vogix-plymouth "$t"
''
