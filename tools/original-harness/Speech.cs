using System;
using System.Reflection;
[assembly: AssemblyVersion("4.0.0.0")]
namespace System.Speech.Synthesis {
  public class Prompt { public Prompt(string s){} public Prompt(PromptBuilder b){} }
  public class PromptBuilder { public void AppendText(string s){} public void AppendSsmlMarkup(string s){} public void StartVoice(string s){} public void EndVoice(){} }
  public enum SynthesizerState { Ready, Speaking, Paused }
  public class VoiceInfo { public string Name {get{return "";}} public System.Globalization.CultureInfo Culture {get{return null;}} }
  public class InstalledVoice { public VoiceInfo VoiceInfo {get{return new VoiceInfo();}} public bool Enabled {get{return true;}} }
  public class SpeechSynthesizer : IDisposable {
    public int Volume {get;set;} public int Rate {get;set;}
    public void SetOutputToWaveFile(string p){} public void SetOutputToDefaultAudioDevice(){} public void SetOutputToNull(){}
    public void Speak(string s){} public void Speak(Prompt p){} public void Speak(PromptBuilder p){} public Prompt SpeakAsync(string s){return null;} public void SpeakAsyncCancelAll(){}
    public void SelectVoice(string s){} public System.Collections.ObjectModel.ReadOnlyCollection<InstalledVoice> GetInstalledVoices(){ return new System.Collections.ObjectModel.ReadOnlyCollection<InstalledVoice>(new InstalledVoice[0]); }
    public VoiceInfo Voice {get{return new VoiceInfo();}}
    public void Dispose(){}
  }
}
