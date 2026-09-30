{ ==========================================================================
  OWLOSUI.PAS - the OWLOSUI toolkit, for a DOS program in Pascal.

  The toolkit is not in here. It is OWLOSRES, the resident, which runs a
  program with the toolkit behind INT 60h; this unit is how a Pascal
  program talks to it. A request is built the way it goes over the wire -
  the bytes of lib/PROTOCOL.md, the same ones the C# client sends down a
  pipe - and a reply comes back:

      Win := OwlWindow(0, -1, -1, 40, 9, OwlDialog, 'Hello', 0);
      OwlStatic(Win, 2, 1, 34, 1, 'Hello, world!');
      OwlButtons(Win, 1);                       a request with parts:
      OwlButton(CmOk, OwlDefault, '~O~K');      begin it, add the parts,
      OwlEnd;                                   and send it

  and a program's main loop is

      repeat OwlWait(Pressed, Command); ... until Done;

  Written for Borland Pascal 7 in real mode (BPC -CD); plain Turbo Pascal
  dialect, nothing Borland-only beyond the Dos unit. Text is sent as it
  is: keep it ASCII, or UTF-8.
  ========================================================================== }

unit OwlosUI;

interface

const
  { window flags }
  OwlModal    = $01;
  OwlFixed    = $02;                    { not resizable }
  OwlNoZoom   = $04;
  OwlNoClose  = $08;
  OwlBlue     = $00;                    { a document }
  OwlCyan     = $10;                    { help }
  OwlGrey     = $20;                    { a dialog }
  OwlDialog   = OwlGrey or OwlFixed or OwlNoZoom;

  { a button's flags }
  OwlDefault  = $01;                    { the one Enter presses }
  OwlAccent   = $02;                    { cyan }
  OwlDanger   = $04;                    { red }
  OwlDisabled = $08;
  OwlCancel   = $10;                    { the one Escape presses }

  { a key, for the status line: kind, value, mods }
  KeyChar     = 0;                      { value: the character }
  KeyF        = 1;                      { value: 1 for F1 }
  KeyNamed    = 2;                      { 0 Enter 1 Esc 2 Tab .. 7 Home 9 PgUp 10 PgDn 11 Up 12 Down 13 Left 14 Right }
  KeyNone     = $FF;
  ModShift    = 1;
  ModCtrl     = 2;
  ModAlt      = 4;

  { what an editor offers (OwlEditor): the core then runs each by itself }
  OfferEdit     = $01;                  { Undo, Redo, Cut, Copy, Paste, Select all }
  OfferFind     = $02;                  { Find... Ctrl+F, Find next Ctrl+L }
  OfferReplace  = $04;                  { Replace... Ctrl+H }
  OfferWrap     = $08;                  { Word wrap }
  OfferReadOnly = $10;                  { Read only }
  OfferHex      = $20;                  { the same text as bytes }
  OfferKeys     = $40;                  { Classic keys: Borland's WordStar arrangement }
  OfferAll      = $7F;
  { and how it is set }
  EditWrap      = $01;
  EditReadOnly  = $02;
  EditClassic   = $04;
  EditHex       = $08;

type
  TGlyphs = array[0..255] of Word;      { each glyph as Unicode }

{ the session }
function  OwlCheck: Boolean;            { started by OWLOSRES? says how, if not }
procedure OwlInit;                      { the desktop: 80 by 25 }
procedure OwlQuit;
procedure OwlWait(var Pressed, Command: Word);

{ views }
function  OwlWindow(Parent: Word; X, Y, W, H: Integer; Flags: Byte;
                    const Title: string; CloseCmd: Word): Word;
function  OwlStatic(Parent: Word; X, Y, W, H: Integer; const Text: string): Word;
function  OwlText(Parent: Word; const Text: string): Word;   { an editor filling it }
function  OwlCanvas(Parent: Word; X, Y, W, H: Integer): Word;

{ requests with parts: begin, add, OwlEnd sends and gives the id }
procedure OwlButtons(Parent: Word; N: Byte);                 { docked bottom right }
procedure OwlButtonRow(Parent: Word; X, Y: Integer; N: Byte);{ placed, not a Tab stop }
procedure OwlMessageBox(const Title, Text: string; N: Byte);
procedure OwlButton(Cmd: Word; Flags: Byte; const Lab: string);
procedure OwlStatus(N: Byte);
procedure OwlWindowStatus(Window: Word; N: Byte);            { keys the window carries }
procedure OwlKey(const Lab: string; Cmd: Word; Kind: Byte; Value: Word; Mods: Byte);
procedure OwlMenuBar(Menus: Byte);
procedure OwlMenu(const Text: string; Items: Byte);
procedure OwlItem(const Text, Shortcut, Hint: string; Cmd: Word);
procedure OwlLine;
procedure OwlBlit(Canvas: Word; X, Y, W, H: Integer);
procedure OwlCell(Ch: Word; Attr: Byte);                     { Ch: a Unicode character }
function  OwlEnd: Word;

{ changing things }
procedure OwlSetText(Id: Word; const Text: string);
procedure OwlEditor(Text: Word; Offers, State: Byte);   { Offer.. bits; Edit.. bits }
function  OwlEditorState(Text: Word): Byte;              { Edit.. bits as the person left them }
procedure OwlClose(Id: Word);
procedure OwlActivate(Id: Word);
function  OwlActive: Word;
procedure OwlZoom(Id: Word);
procedure OwlCycle;
procedure OwlCycleBack;
procedure OwlCascade;
procedure OwlTile;
procedure OwlWindowList;
function  OwlClick(Canvas: Word; var X, Y: Integer): Boolean;{ True if it was clicked }
procedure OwlGlyphs(var Table: TGlyphs);

{ files: DOS reads the folder (FindFirst, FindNext), the panel shows it }
function  OwlFiles(Parent: Word; Top: Integer; const Dir: string): Word;  { top rows left free }
function  OwlSetFiles(Panel: Word; const Dir: string): Boolean;           { False: DOS cannot read it }
procedure OwlFilesError(Panel: Word; const Text: string);
function  OwlTakeFiles(Panel: Word; var Text: string): Byte;  { 0 nothing, 1 a name entered, 2 a path typed }
function  OwlCurrentName(Panel: Word; var Name: string): Boolean;         { the name under the cursor }
function  OwlTextDos(Parent: Word; var Bytes; N: Word; Flags: Byte): Word;
                                        { an editor holding bytes in DOS's code page; Flags 1 = read-only }

{ the wire itself, for what the above does not cover }
procedure OwlBegin(Op: Byte);
procedure OwlU8(V: Byte);
procedure OwlU16(V: Word);
procedure OwlStr(const S: string);
function  OwlSend: Byte;                { the status: 0 OK }

implementation

uses Dos;

var
  Req: array[0..32767] of Byte;         { the request being built: a file's text fits }
  DosTable: TGlyphs;                    { the glyphs as Unicode, for text from DOS }
  DosTableLoaded: Boolean;
  Rep: array[0..2047] of Byte;          { the reply }
  At: Word;                             { where the next byte of Req goes }

{ ---- the wire ---- }

procedure OwlBegin(Op: Byte);
begin
  Req[0] := Op;
  At := 3;
end;

procedure OwlU8(V: Byte);
begin
  Req[At] := V;
  Inc(At);
end;

procedure OwlU16(V: Word);
begin
  Req[At] := Lo(V);
  Req[At + 1] := Hi(V);
  Inc(At, 2);
end;

procedure OwlStr(const S: string);
begin
  OwlU16(Length(S));
  if Length(S) > 0 then Move(S[1], Req[At], Length(S));
  Inc(At, Length(S));
end;

{ DS:SI the request, ES:DI room for the reply, CX how much: INT 60h. }
function OwlSend: Byte;
var
  R: Registers;
  Len: Word;
begin
  Len := At - 3;
  Req[1] := Lo(Len);
  Req[2] := Hi(Len);
  R.DS := Seg(Req);
  R.SI := Ofs(Req);
  R.ES := Seg(Rep);
  R.DI := Ofs(Rep);
  R.CX := SizeOf(Rep);
  Intr($60, R);
  OwlSend := Lo(R.AX);
end;

function ReplyU16(I: Integer): Word;
begin
  ReplyU16 := Rep[3 + I] or (Word(Rep[4 + I]) shl 8);
end;

function SendId: Word;
begin
  OwlSend;
  SendId := ReplyU16(0);
end;

procedure Rect(X, Y, W, H: Integer);
begin
  OwlU16(X); OwlU16(Y); OwlU16(W); OwlU16(H);
end;

procedure Simple(Op: Byte);
begin
  OwlBegin(Op);
  OwlSend;
end;

procedure WithId(Op: Byte; Id: Word);
begin
  OwlBegin(Op);
  OwlU16(Id);
  OwlSend;
end;

{ ---- the session ---- }

{ OWLOSRES's handler has 'OWLOSUI' and a zero in the eight bytes before it. }
function OwlCheck: Boolean;
const
  Signature: array[0..7] of Char = 'OWLOSUI'#0;
var
  P: Pointer;
  I: Integer;
  Ok: Boolean;
begin
  GetIntVec($60, P);
  Ok := Ofs(P^) >= 8;
  if Ok then
    for I := 0 to 7 do
      if Mem[Seg(P^):Ofs(P^) - 8 + I] <> Ord(Signature[I]) then Ok := False;
  if not Ok then
    WriteLn('This program draws with OWLOSUI: start it with OWLOSRES, as RUN.BAT does.');
  OwlCheck := Ok;
end;

procedure OwlInit;
begin
  OwlBegin($01); OwlU16(80); OwlU16(25);
  OwlSend;
end;

procedure OwlQuit;
begin
  Simple($00);
end;

procedure OwlWait(var Pressed, Command: Word);
begin
  Simple($5C);
  Pressed := ReplyU16(0);
  Command := ReplyU16(2);
end;

{ ---- views ---- }

function OwlWindow(Parent: Word; X, Y, W, H: Integer; Flags: Byte;
                   const Title: string; CloseCmd: Word): Word;
begin
  OwlBegin($10); OwlU16(Parent); Rect(X, Y, W, H); OwlU8(Flags); OwlStr(Title); OwlU16(CloseCmd);
  OwlWindow := SendId;
end;

function OwlStatic(Parent: Word; X, Y, W, H: Integer; const Text: string): Word;
begin
  OwlBegin($12); OwlU16(Parent); Rect(X, Y, W, H); OwlStr(Text);
  OwlStatic := SendId;
end;

function OwlText(Parent: Word; const Text: string): Word;
begin
  OwlBegin($11); OwlU16(Parent); Rect(0, 0, 0, 0); OwlU8(0); OwlU8(0); OwlStr(Text);
  OwlText := SendId;
end;

function OwlCanvas(Parent: Word; X, Y, W, H: Integer): Word;
begin
  OwlBegin($1B); OwlU16(Parent); Rect(X, Y, W, H);
  OwlCanvas := SendId;
end;

{ ---- requests with parts ---- }

procedure OwlButtons(Parent: Word; N: Byte);
begin
  OwlBegin($14); OwlU16(Parent); OwlU8(N);
end;

procedure OwlButtonRow(Parent: Word; X, Y: Integer; N: Byte);
begin
  OwlBegin($1E); OwlU16(Parent); Rect(X, Y, 0, 0); OwlU8(1); OwlU8(N);
end;

procedure OwlMessageBox(const Title, Text: string; N: Byte);
begin
  OwlBegin($15); OwlStr(Title); OwlStr(Text); OwlU8(N);
end;

procedure OwlButton(Cmd: Word; Flags: Byte; const Lab: string);
begin
  OwlU16(Cmd); OwlU8(Flags); OwlStr(Lab);
end;

procedure OwlStatus(N: Byte);
begin
  OwlBegin($16); OwlU8(N);
end;

procedure OwlWindowStatus(Window: Word; N: Byte);
begin
  OwlBegin($4A); OwlU16(Window); OwlU8(N);
end;

procedure OwlKey(const Lab: string; Cmd: Word; Kind: Byte; Value: Word; Mods: Byte);
begin
  OwlU16(Cmd); OwlU8(Kind); OwlU16(Value); OwlU8(Mods); OwlStr(Lab);
end;

procedure OwlMenuBar(Menus: Byte);
begin
  OwlBegin($1C); OwlU8(Menus);
end;

procedure OwlMenu(const Text: string; Items: Byte);
begin
  OwlU8(0); OwlU16(0); OwlStr(Text); OwlStr(''); OwlStr(''); OwlU8(Items);
end;

procedure OwlItem(const Text, Shortcut, Hint: string; Cmd: Word);
begin
  OwlU8(0); OwlU16(Cmd); OwlStr(Text); OwlStr(Shortcut); OwlStr(Hint); OwlU8(0);
end;

procedure OwlLine;
begin
  OwlU8(1); OwlU16(0); OwlStr(''); OwlStr(''); OwlStr(''); OwlU8(0);
end;

procedure OwlBlit(Canvas: Word; X, Y, W, H: Integer);
begin
  OwlBegin($2C); OwlU16(Canvas); Rect(X, Y, W, H);
end;

procedure OwlCell(Ch: Word; Attr: Byte);
begin
  OwlU16(Ch); OwlU8(Attr);
end;

function OwlEnd: Word;
begin
  OwlEnd := SendId;
end;

{ ---- changing things ---- }

procedure OwlSetText(Id: Word; const Text: string);
begin
  OwlBegin($2B); OwlU16(Id); OwlStr(Text);
  OwlSend;
end;

procedure OwlEditor(Text: Word; Offers, State: Byte);
begin
  OwlBegin($5D); OwlU16(Text); OwlU8(Offers); OwlU8(State);
  OwlSend;
end;

function OwlEditorState(Text: Word): Byte;
begin
  OwlBegin($5E); OwlU16(Text);
  OwlSend;
  OwlEditorState := Rep[4];
end;

procedure OwlClose(Id: Word);    begin WithId($20, Id); end;
procedure OwlActivate(Id: Word); begin WithId($27, Id); end;
procedure OwlZoom(Id: Word);     begin WithId($44, Id); end;
procedure OwlCycle;              begin Simple($43); end;
procedure OwlCycleBack;          begin Simple($4D); end;
procedure OwlCascade;            begin Simple($47); end;
procedure OwlTile;               begin Simple($48); end;
procedure OwlWindowList;         begin Simple($4C); end;

function OwlActive: Word;
begin
  OwlBegin($2A);
  OwlActive := SendId;
end;

function OwlClick(Canvas: Word; var X, Y: Integer): Boolean;
begin
  WithId($2F, Canvas);
  OwlClick := Rep[3] <> 0;
  X := Integer(ReplyU16(1));
  Y := Integer(ReplyU16(3));
end;

procedure OwlGlyphs(var Table: TGlyphs);
var
  I: Integer;
begin
  Simple($42);
  for I := 0 to 255 do Table[I] := ReplyU16(3 + I * 2);
end;

{ ---- files: DOS reads the folder and the file, the panel shows them ---- }

const
  AnyEntry = Directory or ReadOnly or Hidden or SysFile or Archive;  { no volume label }
  NoMoreFiles = 18;

type
  TBytes = array[0..65534] of Byte;

{ A folder made into DOS's pattern for everything in it: C:\DIR\*.* }
function PatternOf(const Dir: string): string;
begin
  if (Length(Dir) = 0) or (Dir[Length(Dir)] <> '\') then PatternOf := Dir + '\*.*'
  else PatternOf := Dir + '*.*';
end;

procedure PatchU16(Where, V: Word);
begin
  Req[Where] := Lo(V);
  Req[Where + 1] := Hi(V);
end;

{ The folder's entries after a request's header, as FILES, SET_FILES and
  ADD_FILES want them: a count, then a name, size, date, time and DOS
  attributes each, straight from FindFirst and FindNext. '.' is left out;
  '..' is kept, as DOS gives it. A folder too big for one request goes on
  in ADD_FILES to the same panel - whose id, for a new panel, is the reply
  to the first part. }
function Listing(Panel: Word; var SR: SearchRec; IsNew: Boolean): Word;
var
  CountAt, Count, D, T: Word;
begin
  CountAt := At;
  Count := 0;
  OwlU16(0);
  while DosError = 0 do
  begin
    if SR.Name <> '.' then
    begin
      if At + 64 > SizeOf(Req) then
      begin
        PatchU16(CountAt, Count);
        OwlSend;
        if IsNew then begin Panel := ReplyU16(0); IsNew := False; end;
        OwlBegin($5A); OwlU16(Panel);
        CountAt := At;
        Count := 0;
        OwlU16(0);
      end;
      OwlStr(SR.Name);
      OwlU16(Word(SR.Size)); OwlU16(Word(SR.Size shr 16));
      D := SR.Time shr 16;
      T := SR.Time and $FFFF;
      OwlU16(1980 + D shr 9);
      OwlU8((D shr 5) and 15); OwlU8(D and 31);
      OwlU8(T shr 11); OwlU8((T shr 5) and 63);
      OwlU8(SR.Attr);
      Inc(Count);
    end;
    FindNext(SR);
  end;
  PatchU16(CountAt, Count);
  OwlSend;
  if IsNew then Listing := ReplyU16(0) else Listing := Panel;
end;

function OwlFiles(Parent: Word; Top: Integer; const Dir: string): Word;
var
  SR: SearchRec;
  Pattern: string;
begin
  Pattern := PatternOf(Dir);
  FindFirst(Pattern, AnyEntry, SR);
  OwlBegin($1A); OwlU16(Parent); Rect(0, Top, 0, 0); OwlU8(0); OwlStr('*.*'); OwlStr(Pattern);
  OwlFiles := Listing(0, SR, True);     { a folder DOS cannot read: an empty panel }
end;

function OwlSetFiles(Panel: Word; const Dir: string): Boolean;
var
  SR: SearchRec;
  Pattern: string;
begin
  Pattern := PatternOf(Dir);
  FindFirst(Pattern, AnyEntry, SR);
  if (DosError <> 0) and (DosError <> NoMoreFiles) then
  begin
    OwlSetFiles := False;               { no such folder, or no disk }
    Exit;
  end;
  OwlBegin($25); OwlU16(Panel); OwlStr(Pattern); OwlStr('*.*');
  Listing(Panel, SR, False);
  OwlSetFiles := True;
end;

procedure OwlFilesError(Panel: Word; const Text: string);
begin
  OwlBegin($29); OwlU16(Panel); OwlStr(Text);
  OwlSend;
end;

function OwlTakeFiles(Panel: Word; var Text: string): Byte;
var
  N: Word;
begin
  WithId($26, Panel);
  N := ReplyU16(1);
  if N > 255 then N := 255;
  Text[0] := Chr(N);
  if N > 0 then Move(Rep[6], Text[1], N);
  OwlTakeFiles := Rep[3];
end;

function OwlCurrentName(Panel: Word; var Name: string): Boolean;
var
  N: Word;
begin
  WithId($28, Panel);
  OwlCurrentName := ReplyU16(0) <> 0;
  N := ReplyU16(2);
  if N > 255 then N := 255;
  Name[0] := Chr(N);
  if N > 0 then Move(Rep[7], Name[1], N);
end;

{ Bytes in the machine's code page, as an editor: each byte through the
  glyph table into Unicode and on as UTF-8, which is what the wire
  carries. A line ends at LF; CR is dropped; a tab is a space. What does
  not fit in one request is left out. }
function OwlTextDos(Parent: Word; var Bytes; N: Word; Flags: Byte): Word;
var
  LenAt, I, C: Word;
  B: Byte;
begin
  if not DosTableLoaded then
  begin
    OwlGlyphs(DosTable);
    DosTableLoaded := True;
  end;
  OwlBegin($11); OwlU16(Parent); Rect(0, 0, 0, 0); OwlU8(0); OwlU8(Flags);
  LenAt := At;
  OwlU16(0);
  I := 0;
  while (I < N) and (At + 4 < SizeOf(Req)) do
  begin
    B := TBytes(Bytes)[I];
    Inc(I);
    if B <> 13 then
    begin
      if B = 10 then C := 10
      else if B = 9 then C := 32
      else C := DosTable[B];
      if C < $80 then OwlU8(C)
      else if C < $800 then
      begin
        OwlU8($C0 or (C shr 6));
        OwlU8($80 or (C and $3F));
      end
      else
      begin
        OwlU8($E0 or (C shr 12));
        OwlU8($80 or ((C shr 6) and $3F));
        OwlU8($80 or (C and $3F));
      end;
    end;
  end;
  PatchU16(LenAt, At - LenAt - 2);
  OwlTextDos := SendId;
end;

end.
