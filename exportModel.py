import torch
import timm
from huggingface_hub import hf_hub_download

# Download the model
model_path = hf_hub_download(
    repo_id="ntsrigaud/hagrid-vit-gesture",
    filename="best_model.pth"
)

# Load it
model = timm.create_model('vit_base_patch16_224', pretrained=False, num_classes=24)
checkpoint = torch.load(model_path, map_location='cpu')

if 'model_state_dict' in checkpoint:
    model.load_state_dict(checkpoint['model_state_dict'])
else:
    model.load_state_dict(checkpoint)

model.eval()

# Export to ONNX
dummy_input = torch.randn(1, 3, 224, 224)  # matches training input size

torch.onnx.export(
    model,
    dummy_input,
    "hagrid_vit_gesture.onnx",
    input_names=["input"],
    output_names=["output"],
    dynamic_axes={"input": {0: "batch_size"}, "output": {0: "batch_size"}},
    opset_version=17
)

print("Exported successfully!")