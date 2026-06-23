import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { GestureOverlay } from "./GestureOverlay";
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <main className="container">
      <GestureOverlay />
      <h1>Hi, I'm Jarvis</h1>

      <div className="row">
      </div>
      <p></p>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input
          id="greet-input"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="What can I assist you with..."
        />
        <button type="submit">Query</button>
      </form>
      <p>{greetMsg}</p>
    </main>
  );
}

export default App;
