import * as THREE from '../vendor/three.module.js';
import { RectAreaLightUniformsLib } from '../vendor/RectAreaLightUniformsLib.js';
import { createWinterExterior } from './winter.js';

const $ = (id) => document.getElementById(id);
try { init(); } catch (error) { console.error(error); $('error').hidden = false; }

function init() {
const scene = new THREE.Scene();
scene.background = new THREE.Color('#a6b4bf');
// Distance haze belongs beyond the windows; keep the entrance air clear.
scene.fog = new THREE.Fog('#a6b4bf', 9, 42);
RectAreaLightUniformsLib.init();
const renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: 'high-performance' });
renderer.setSize(innerWidth, innerHeight);
renderer.setPixelRatio(Math.min(devicePixelRatio, 1.75));
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;
// Furniture and lights stay in place; reuse their shadows while walking.
renderer.shadowMap.autoUpdate = false;
renderer.shadowMap.needsUpdate = true;
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1.15;
$('house').appendChild(renderer.domElement);
const camera = new THREE.PerspectiveCamera(65, innerWidth / innerHeight, .05, 100);
camera.position.set(.8, 1.65, 3.8);
camera.rotation.order = 'YXZ';
let yaw = .11, pitch = -.035;
const colliders = [], targets = [];
const mat = (color, roughness = .8, metalness = 0) => new THREE.MeshStandardMaterial({ color, roughness, metalness });
const wood = mat('#4b3020'), darkWood = mat('#291e15'), trim = mat('#393e32'), wall = mat('#737966'), brass = mat('#ad8a49', .32, .7);
const cream = mat('#d4c9aa');
function box(w,h,d,material,x,y,z, parent=scene) {
  const mesh = new THREE.Mesh(new THREE.BoxGeometry(w,h,d), material);
  mesh.position.set(x,y,z); mesh.castShadow=true; mesh.receiveShadow=true; parent.add(mesh); return mesh;
}
function cylinder(rt,rb,h,material,x,y,z,parent=scene) {
  const mesh = new THREE.Mesh(new THREE.CylinderGeometry(rt,rb,h,32),material);
  mesh.position.set(x,y,z);mesh.castShadow=true;mesh.receiveShadow=true;parent.add(mesh);return mesh;
}
function sphere(r,material,x,y,z,parent=scene) {
  const mesh = new THREE.Mesh(new THREE.SphereGeometry(r,20,14),material);
  mesh.position.set(x,y,z);mesh.castShadow=true;parent.add(mesh);return mesh;
}
function block(x,z,w,d) { colliders.push({x,z,w:w/2+.23,d:d/2+.23}); }
function canvasTexture(draw,w=512,h=512) {
  const canvas=document.createElement('canvas');canvas.width=w;canvas.height=h;
  draw(canvas.getContext('2d'),w,h);
  const tex=new THREE.CanvasTexture(canvas);tex.colorSpace=THREE.SRGBColorSpace;tex.anisotropy=renderer.capabilities.getMaxAnisotropy();return tex;
}
// Individually laid floorboards with grain, knots and staggered seams.
const floorTexture=canvasTexture((c,w,h)=>{
  c.fillStyle='#543e2c';c.fillRect(0,0,w,h);
  for(let row=0;row<8;row++)for(let col=-1;col<3;col++){
    const x=col*256+(row%2)*128,y=row*64;
    const v=45+Math.random()*15;c.fillStyle=`hsl(28,27%,${v-20}%)`;c.fillRect(x+1,y+1,254,62);
    for(let i=0;i<50;i++){c.strokeStyle=`rgba(20,12,4,${Math.random()*.15})`;c.beginPath();const gy=y+Math.random()*64;c.moveTo(x,gy);c.bezierCurveTo(x+80,gy-3,x+170,gy+3,x+256,gy);c.stroke();}
  }
});
floorTexture.wrapS=floorTexture.wrapT=THREE.RepeatWrapping;floorTexture.repeat.set(3,3.5);
box(10,.15,12,new THREE.MeshStandardMaterial({map:floorTexture,roughness:.62}),0,-.08,0);
box(10,.2,12,mat('#b0ad94'),0,3.9,0);
box(10,3.9,.18,wall,0,1.95,-6);
box(10,3.9,.18,wall,0,1.95,6);
box(.18,3.9,12,wall,5,1.95,0);
// Left wall, with a real opening for the winter window.
box(.18,3.9,3,wall,-5,1.95,-4.5);box(.18,3.9,4,wall,-5,1.95,4);
box(.18,.9,5,wall,-5,.45,-.5);box(.18,.65,5,wall,-5,3.575,-.5);
// Wainscoting and architectural trim.
for(const x of [-4.87,4.87]){
  box(.12,1.05,11.8,trim,x,.525,0);box(.18,.07,11.8,darkWood,x,1.08,0);
  box(.19,.16,11.8,darkWood,x,.08,0);box(.23,.13,11.8,trim,x,3.75,0);
  for(let z=-5.7;z<6;z+=.85)box(.16,.85,.035,mat('#4b5040'),x+(x<0?.07:-.07),.54,z);
}
for(const z of [-5.88,5.88]){
  box(9.8,1.05,.1,trim,0,.525,z);box(9.8,.07,.17,darkWood,0,1.08,z);
  box(9.8,.16,.16,darkWood,0,.08,z);box(9.8,.13,.23,trim,0,3.75,z);
  for(let x=-4.5;x<5;x+=.85)box(.035,.85,.17,mat('#4b5040'),x,.54,z);
}
// Window: cold glass, mullions, sill, and the winter forest outdoors.
const glass=new THREE.MeshPhysicalMaterial({color:'#c1d4dc',roughness:.16,metalness:0,transparent:true,opacity:.12,depthWrite:false,clearcoat:1});
const windowPane=box(.035,2.3,4.9,glass,-5,2.05,-.5);
windowPane.castShadow=false;windowPane.renderOrder=3;
for(const z of [-3,2])box(.25,2.48,.12,cream,-4.87,2.05,z);
for(const y of [.9,3.2])box(.25,.12,5.15,cream,-4.87,y,-.5);
for(const z of [-1.35,.35])box(.16,2.3,.065,cream,-4.85,2.05,z);
box(.16,.065,5,cream,-4.84,2,-.5);box(.48,.1,5.35,cream,-4.76,.92,-.5);
const winter=createWinterExterior(scene,canvasTexture);
const frostTexture=canvasTexture((c,w,h)=>{
  for(let i=0;i<6500;i++){
    const x=Math.random()*w,y=Math.random()*h;
    const edge=Math.min(x/w,1-x/w,y/h,1-y/h);
    if(edge>.085*Math.random())continue;
    c.strokeStyle=`rgba(227,239,244,${.12+Math.random()*.35})`;
    const size=1+Math.random()*6;c.lineWidth=.6;
    for(let a=0;a<3;a++){
      const angle=a*Math.PI/3;c.beginPath();c.moveTo(x-Math.cos(angle)*size,y-Math.sin(angle)*size);c.lineTo(x+Math.cos(angle)*size,y+Math.sin(angle)*size);c.stroke();
    }
  }
},1024,512);
const frost=new THREE.Mesh(new THREE.PlaneGeometry(4.9,2.3),new THREE.MeshBasicMaterial({map:frostTexture,transparent:true,depthWrite:false,opacity:.75}));
frost.rotation.y=Math.PI/2;frost.position.set(-4.97,2.05,-.5);frost.renderOrder=4;scene.add(frost);
box(.28,.075,5.12,mat('#d9e3e7'),-5.16,.96,-.5);
const curtain=mat('#8c8a70');
for(const z of [-3.1,2.1])for(let i=0;i<8;i++)cylinder(.09,.12,2.7,curtain,-4.55,1.92,z+(i-3.5)*.055);
const rod=cylinder(.035,.035,5.8,brass,-4.54,3.36,-.5);rod.rotation.x=Math.PI/2;

function label(text,sub=''){
  return canvasTexture((c,w,h)=>{c.fillStyle='#8d7549';c.fillRect(0,0,w,h);c.strokeStyle='#d3b982';c.lineWidth=4;c.strokeRect(10,10,w-20,h-20);c.textAlign='center';c.fillStyle='#f4e5bc';c.font='38px Georgia';c.fillText(text,w/2,sub?68:82);if(sub){c.font='16px sans-serif';c.fillStyle='#e1cda3';c.fillText(sub,w/2,105);}},512,150);
}
function door(name,x,z,rotation=0) {
  const g=new THREE.Group();g.position.set(x,0,z);g.rotation.y=rotation;scene.add(g);
  box(1.65,2.8,.17,darkWood,0,1.4,0,g);
  const panel=box(1.42,2.63,.1,wood,0,1.315,.12,g);
  for(const y of [.61,1.65]){
    box(1.14,y===.61?.73:1.05,.05,darkWood,0,y,.19,g);
    box(1.02,y===.61?.61:.93,.055,wood,0,y,.22,g);
  }
  for(const dx of [-.84,.84])box(.15,2.95,.28,cream,dx,1.475,.06,g);
  box(1.85,.16,.28,cream,0,2.9,.06,g);
  box(1.48,.035,.35,brass,0,.025,.14,g);
  box(.13,.3,.04,brass,.48,1.2,.25,g);sphere(.065,brass,.48,1.24,.32,g);
  const sign=box(.9,.265,.035,new THREE.MeshStandardMaterial({map:label(name)}),0,2.38,.27,g);
  const action={label:`${name} · E to try the door`,run:()=>toast(`${name} — this room isn’t open yet. For now, make yourself at home in the entrance hall.`)};
  panel.userData.action=action;sign.userData.action=action;targets.push(panel,sign);
  return g;
}
door('LIVING ROOM',-2.5,-5.75);
door('KITCHEN',1,-5.75);
door('STUDY',4.75,-2, -Math.PI/2);
door('FRONT DOOR',.8,5.75,Math.PI);
// Runner rug, woven border and repeating medallions.
const rugTexture=canvasTexture((c,w,h)=>{
 c.fillStyle='#50332c';c.fillRect(0,0,w,h);c.strokeStyle='#aa8b5a';c.lineWidth=12;c.strokeRect(20,20,w-40,h-40);c.lineWidth=3;c.strokeRect(42,42,w-84,h-84);
 for(let y=110;y<h-50;y+=155){c.save();c.translate(w/2,y);c.scale(1,.7);c.rotate(Math.PI/4);c.fillStyle='#a88b60';c.fillRect(-55,-55,110,110);c.fillStyle='#4e5550';c.fillRect(-36,-36,72,72);c.fillStyle='#b39b70';c.fillRect(-12,-12,24,24);c.restore();}
 for(let i=0;i<12000;i++){c.fillStyle=Math.random()>.5?'#ffffff07':'#0000000b';c.fillRect(Math.random()*w,Math.random()*h,1,3);}
},512,1024);
box(2.5,.025,6,new THREE.MeshStandardMaterial({map:rugTexture,roughness:1}),.15,.02,-.7);
for(let i=0;i<60;i++){box(.013,.009,.13,mat('#9b8e6e'),-1.08+i*.041,.025,2.36);box(.013,.009,.13,mat('#9b8e6e'),-1.08+i*.041,.025,-3.76);}
// Console table on the right-hand wall.
box(.67,.13,2.4,wood,4.37,.85,1.6);box(.59,.22,2.25,darkWood,4.37,.69,1.6);
for(const x of [4.12,4.6])for(const z of [.55,2.64])box(.075,.72,.075,wood,x,.36,z);
box(.5,.07,2.1,wood,4.37,.22,1.6);block(4.37,1.6,.8,2.5);
for(const z of [1.03,2.1])sphere(.035,brass,4.04,.69,z);
// A framed mirror above the console.
box(.1,1.55,1.2,brass,4.82,2.12,1.6);
box(.06,1.4,1.05,new THREE.MeshStandardMaterial({color:'#78918b',metalness:.85,roughness:.22}),4.75,2.12,1.6);
// Woven linen catches light throughout its thickness. The emissive map
// describes the bulb's diffusion through fabric, not a flat self-lit cover.
cylinder(.17,.22,.06,brass,4.32,.96,2.23);cylinder(.035,.05,.48,brass,4.32,1.21,2.23);
const linen=canvasTexture((c,w,h)=>{
  c.fillStyle='#dbcfb5';c.fillRect(0,0,w,h);
  for(let x=0;x<w;x+=3){c.strokeStyle=x%9===0?'#a99c8544':'#fff5d955';c.lineWidth=.7;c.beginPath();c.moveTo(x,0);c.lineTo(x,h);c.stroke();}
  for(let y=0;y<h;y+=3){c.strokeStyle=y%9===0?'#8b806133':'#fff5d944';c.lineWidth=.65;c.beginPath();c.moveTo(0,y);c.lineTo(w,y);c.stroke();}
});
const diffusion=canvasTexture((c,w,h)=>{
  const glow=c.createLinearGradient(0,0,0,h);
  glow.addColorStop(0,'#6b6b6b');glow.addColorStop(.16,'#bebebe');glow.addColorStop(.5,'#ffffff');glow.addColorStop(.8,'#c5c5c5');glow.addColorStop(1,'#757575');
  c.fillStyle=glow;c.fillRect(0,0,w,h);
  // Support ribs and weave slightly interrupt the transmitted light.
  for(let x=0;x<w;x+=w/8){c.fillStyle='#0000000c';c.fillRect(x,0,3,h);}
  for(let y=0;y<h;y+=3){c.fillStyle='#00000008';c.fillRect(0,y,w,1);}
});
const shadeMaterial=new THREE.MeshPhysicalMaterial({map:linen,bumpMap:linen,bumpScale:.0015,roughness:.95,sheen:.65,sheenColor:new THREE.Color('#f5e3c5'),emissive:'#ffd29b',emissiveMap:diffusion,emissiveIntensity:1.3,side:THREE.DoubleSide});
const shade=new THREE.Mesh(new THREE.CylinderGeometry(.19,.33,.42,64,1,true),shadeMaterial);shade.position.set(4.32,1.58,2.23);shade.receiveShadow=true;scene.add(shade);
// The fabric transmits light; an opaque shadow from the shade would hide it.
shade.castShadow=false;
const bulb=sphere(.063,new THREE.MeshBasicMaterial({color:'#fff0cf'}),4.32,1.57,2.23);bulb.castShadow=false;
for(const [radius,y] of [[.19,1.79],[.33,1.37]]){
  const piping=new THREE.Mesh(new THREE.TorusGeometry(radius,.007,8,64),new THREE.MeshStandardMaterial({color:'#b9a683',roughness:.9}));piping.rotation.x=Math.PI/2;piping.position.set(4.32,y,2.23);scene.add(piping);
}
const diffusedLights=[];
for(let i=0;i<4;i++){
  const angle=i*Math.PI/2;
  const light=new THREE.RectAreaLight('#ffcf96',9,.29,.32);
  light.position.set(4.32+Math.cos(angle)*.35,1.58,2.23+Math.sin(angle)*.35);
  light.lookAt(4.32+Math.cos(angle),1.58,2.23+Math.sin(angle));scene.add(light);diffusedLights.push(light);
}
function lampSpill(y,targetY,intensity,angle){
  const light=new THREE.SpotLight('#ffce92',intensity,5,angle,.85,2);
  light.position.set(4.32,y,2.23);light.target.position.set(4.32,targetY,2.23);
  light.castShadow=true;light.shadow.mapSize.set(512,512);light.shadow.normalBias=.025;light.shadow.bias=-.0001;
  scene.add(light,light.target);return light;
}
const lampDown=lampSpill(1.34,.8,4.2,1.05),lampUp=lampSpill(1.82,3.8,3.2,.9);
// A small fill approximates the light reflected back from the nearby wall.
const lamp=new THREE.PointLight('#ffd9a6',1.7,3.5,2);lamp.position.set(4.1,1.55,2.23);scene.add(lamp);
let lampOn=true;shade.userData.action={label:'Table lamp · E to switch',run:()=>{
  lampOn=!lampOn;
  diffusedLights.forEach(light=>light.intensity=lampOn?9:0);
  lampDown.intensity=lampOn?4.2:0;lampUp.intensity=lampOn?3.2:0;lamp.intensity=lampOn?1.7:0;
  shadeMaterial.emissiveIntensity=lampOn?1.3:0;bulb.visible=lampOn;
  toast(lampOn?'A little warmth.':'The lamp is off.');
}};targets.push(shade);
// Books, a ceramic bowl, and a small note.
for(let i=0;i<3;i++)box(.34,.055,.5,mat(['#626950','#6d4031','#b0a07b'][i]),4.28,.965+i*.055,.85);
const bowl=new THREE.Mesh(new THREE.SphereGeometry(.17,24,12,0,Math.PI*2,0,Math.PI*.52),mat('#88958c'));bowl.rotation.x=Math.PI; bowl.position.set(4.27,1.09,1.4);scene.add(bowl);
const note=box(.28,.008,.35,new THREE.MeshStandardMaterial({map:canvasTexture((c,w,h)=>{c.fillStyle='#d4c4a0';c.fillRect(0,0,w,h);c.fillStyle='#4a493a';c.textAlign='center';c.font='italic 40px Georgia';c.fillText('Welcome home.',w/2,190);c.font='25px Georgia';c.fillText('Leave the weather outside.',w/2,270);})}),4.22,.926,1.76);
note.userData.action={label:'Welcome note · E to read',run:()=>toast('“Welcome home. Leave the weather outside. There’s no hurry here.”')};targets.push(note);
// Upholstered window bench.
box(.88,.16,2,wood,-4.02,.43,-.5);box(.92,.15,2.06,mat('#8e896e'),-4.02,.575,-.5);
for(const z of [-1.32,.32])for(const x of [-4.32,-3.72])box(.07,.4,.07,wood,x,.2,z);
block(-4.02,-.5,1,2.2);
const cushion=box(.57,.18,.56,mat('#a99b78'),-4.05,.73,-1.07);cushion.rotation.y=.2;
// Coat stand and a hanging wool coat.
cylinder(.26,.32,.09,darkWood,3.7,.05,4.4);cylinder(.045,.065,2.15,wood,3.7,1.1,4.4);
for(let i=0;i<4;i++){const a=i*Math.PI/2;const hook=box(.5,.035,.035,brass,3.7+Math.cos(a)*.17,1.94,4.4+Math.sin(a)*.17);hook.rotation.y=-a;}
box(.42,.95,.22,mat('#545a4c'),3.82,1.34,4.4);block(3.7,4.4,.8,.8);
// Umbrella pot.
cylinder(.2,.15,.52,mat('#625f4c'),-3.7,.26,4.6);block(-3.7,4.6,.45,.45);
for(let i=0;i<3;i++){const umbrella=cylinder(.025,.025,1.03,darkWood,-3.75+i*.06,.68,4.6);umbrella.rotation.z=(i-1)*.12;sphere(.045,brass,-3.75+i*.06,1.2,4.6);}
// Landscape painting, a quiet bit of colour beside the doors.
box(1.4,1.02,.09,brass,3.22,2.1,-5.76);
const painting=canvasTexture((c,w,h)=>{c.fillStyle='#9aab9e';c.fillRect(0,0,w,h);for(let j=0;j<4;j++){c.fillStyle=['#7b8b7b','#5d7367','#425c51','#354d40'][j];c.beginPath();c.moveTo(0,h);for(let x=0;x<=w;x+=8)c.lineTo(x,190+j*65+Math.sin(x*.012+j*2)*42+Math.sin(x*.025)*16);c.lineTo(w,h);c.fill();}c.fillStyle='#c1c0a3';c.beginPath();c.arc(340,110,28,0,Math.PI*2);c.fill();});
box(1.23,.85,.03,new THREE.MeshStandardMaterial({map:painting}),3.22,2.1,-5.7);
// Opal glass diffuses the pendant's bulb. Low ambient light preserves the
// contrast between warm interior pools and the cold overcast window light.
cylinder(.07,.07,.08,brass,0,3.76,-.8);cylinder(.016,.016,.6,darkWood,0,3.45,-.8);
const pendantMaterial=new THREE.MeshPhysicalMaterial({color:'#eee2c9',emissive:'#ffe1ae',emissiveMap:diffusion,emissiveIntensity:1.35,roughness:.38,clearcoat:.3,clearcoatRoughness:.3});
const pendant=new THREE.Mesh(new THREE.SphereGeometry(.28,40,24),pendantMaterial);pendant.position.set(0,3.08,-.8);scene.add(pendant);
const ceilingLight=new THREE.PointLight('#ffdfb3',22,11,2);ceilingLight.position.set(0,3.08,-.8);ceilingLight.castShadow=true;ceilingLight.shadow.mapSize.set(1024,1024);ceilingLight.shadow.bias=-.0001;ceilingLight.shadow.normalBias=.025;ceilingLight.shadow.camera.near=.12;scene.add(ceilingLight);
scene.add(new THREE.HemisphereLight('#cadce9','#6c5a43',.6));
const windowLight=new THREE.RectAreaLight('#ccdfef',5,4.8,2.2);windowLight.position.set(-4.77,2.05,-.5);windowLight.lookAt(0,1.2,-.5);scene.add(windowLight);
// A broad, low-intensity fill stands in for bounced light from the ceiling.
const ceilingBounce=new THREE.RectAreaLight('#ead7b3',.8,5,6);ceilingBounce.position.set(0,3.7,-.8);ceilingBounce.lookAt(0,0,-.8);scene.add(ceilingBounce);

let active=false, started=false, currentTarget=null, toastTimer, soundOn=false, audio, noiseSource, noiseGain, gustOscillator;
const coarse=matchMedia('(pointer: coarse)').matches;
const keys=new Set();
function toast(message){$('toast').textContent=message;$('toast').classList.add('show');clearTimeout(toastTimer);toastTimer=setTimeout(()=>$('toast').classList.remove('show'),4500);}
function setActive(value){active=value;document.body.classList.toggle('exploring',value);$('enter').textContent=started?'Continue exploring →':'Step inside →';$('controls-label').textContent=value?'W A S D to walk · E to interact · Esc to pause':'An unhurried place to explore';if(!value){keys.clear();$('interaction').style.display='none';}}
async function enter(){
 started=true;$('help-panel').hidden=true;$('help').setAttribute('aria-expanded','false');
 if(coarse){setActive(true);return;}
 try{await renderer.domElement.requestPointerLock();}catch{setActive(true);toast('Drag to look around. Use W A S D to walk.');}
}
$('enter').disabled=false;$('enter').textContent='Step inside →';$('enter').addEventListener('click',enter);
document.addEventListener('pointerlockchange',()=>setActive(document.pointerLockElement===renderer.domElement));
document.addEventListener('pointerlockerror',()=>{setActive(true);toast('Drag to look around. Use W A S D to walk.');});
let drag=null;
renderer.domElement.addEventListener('pointerdown',e=>{if(!active)return;if(document.pointerLockElement){interact();return;}drag={x:e.clientX,y:e.clientY,moved:false};renderer.domElement.setPointerCapture(e.pointerId);});
renderer.domElement.addEventListener('pointermove',e=>{if(!active||document.pointerLockElement)return;if(drag){const dx=e.clientX-drag.x,dy=e.clientY-drag.y;yaw-=dx*.004;pitch-=dy*.003;drag.moved=drag.moved||Math.abs(dx)+Math.abs(dy)>2;drag.x=e.clientX;drag.y=e.clientY;}});
renderer.domElement.addEventListener('pointerup',()=>{if(drag&&!drag.moved)interact();drag=null;});
renderer.domElement.style.touchAction='none';
document.addEventListener('mousemove',e=>{if(active&&document.pointerLockElement===renderer.domElement){yaw-=e.movementX*.002;pitch-=e.movementY*.002;}});
const moveCodes={KeyW:'forward',ArrowUp:'forward',KeyS:'backward',ArrowDown:'backward',KeyA:'left',ArrowLeft:'left',KeyD:'right',ArrowRight:'right'};
document.addEventListener('keydown',e=>{if(e.code==='Escape'){setActive(false);return;}if(!active)return;if(moveCodes[e.code]){keys.add(moveCodes[e.code]);e.preventDefault();}if(e.code==='KeyE'&&!e.repeat)interact();});
document.addEventListener('keyup',e=>{if(moveCodes[e.code])keys.delete(moveCodes[e.code]);});
window.addEventListener('blur',()=>{keys.clear();if(active){document.exitPointerLock?.();setActive(false);}});
document.querySelectorAll('[data-move]').forEach(button=>{
 button.addEventListener('pointerdown',e=>{e.preventDefault();button.setPointerCapture(e.pointerId);keys.add(button.dataset.move);});
 for(const event of ['pointerup','pointercancel','lostpointercapture'])button.addEventListener(event,()=>keys.delete(button.dataset.move));
});
$('touch-use').onclick=()=>interact();
$('help').onclick=()=>{if(document.pointerLockElement)document.exitPointerLock();setActive(false);$('help-panel').hidden=!$('help-panel').hidden;$('help').setAttribute('aria-expanded',String(!$('help-panel').hidden));};
$('reset').onclick=()=>{camera.position.set(.8,1.65,3.8);yaw=.11;pitch=-.035;$('help-panel').hidden=true;$('help').setAttribute('aria-expanded','false');toast('Back at the entrance.');};
function interact(){if(active&&currentTarget)currentTarget.userData.action.run();}
$('sound').onclick=async()=>{
 try{
 if(!audio){audio=new (window.AudioContext||window.webkitAudioContext)();const buffer=audio.createBuffer(1,audio.sampleRate*3,audio.sampleRate),data=buffer.getChannelData(0);let last=0;for(let i=0;i<data.length;i++){last=(last+Math.random()*.04-.02)/1.02;data[i]=last*4;}noiseSource=audio.createBufferSource();noiseSource.buffer=buffer;noiseSource.loop=true;const filter=audio.createBiquadFilter();filter.type='lowpass';filter.frequency.value=520;noiseGain=audio.createGain();noiseGain.gain.value=0;noiseSource.connect(filter);filter.connect(noiseGain);noiseGain.connect(audio.destination);noiseSource.start();
 // Slow variations in filtered wind follow the storm's gusting character.
 gustOscillator=audio.createOscillator();gustOscillator.frequency.value=.12;const gustDepth=audio.createGain();gustDepth.gain.value=170;gustOscillator.connect(gustDepth);gustDepth.connect(filter.frequency);gustOscillator.start();}
 await audio.resume();soundOn=!soundOn;noiseGain.gain.setTargetAtTime(soundOn?.3:0,audio.currentTime,.3);$('sound').innerHTML=`Sound ${soundOn?'on':'off'} <span>♫</span>`;$('sound').setAttribute('aria-pressed',String(soundOn));
 }catch{toast('Ambient sound isn’t available in this browser.');}
};
const raycaster=new THREE.Raycaster();raycaster.far=2.8;
function canMove(x,z){return x>-4.5&&x<4.5&&z>-5.4&&z<5.4&&!colliders.some(c=>Math.abs(x-c.x)<c.w&&Math.abs(z-c.z)<c.d);}
const clock=new THREE.Clock();const reducedMotion=matchMedia('(prefers-reduced-motion: reduce)').matches;
function frame(){
 requestAnimationFrame(frame);const dt=Math.min(clock.getDelta(),.05);
 pitch=THREE.MathUtils.clamp(pitch,-1.25,1.25);camera.rotation.set(pitch,yaw,0);
 if(active){
  let forward=Number(keys.has('forward'))-Number(keys.has('backward')),side=Number(keys.has('right'))-Number(keys.has('left'));
  const length=Math.hypot(forward,side)||1;forward/=length;side/=length;
  const speed=dt*2;const dx=(-Math.sin(yaw)*forward+Math.cos(yaw)*side)*speed,dz=(-Math.cos(yaw)*forward-Math.sin(yaw)*side)*speed;
  if(canMove(camera.position.x+dx,camera.position.z))camera.position.x+=dx;
  if(canMove(camera.position.x,camera.position.z+dz))camera.position.z+=dz;
  raycaster.setFromCamera(new THREE.Vector2(0,0),camera);const hits=raycaster.intersectObjects(targets,false);currentTarget=hits[0]?.object||null;
  $('interaction').textContent=currentTarget?currentTarget.userData.action.label:'';$('interaction').style.display=currentTarget?'block':'none';
 }
 if(!reducedMotion)winter.update(dt);
 renderer.render(scene,camera);
}
window.addEventListener('resize',()=>{camera.aspect=innerWidth/innerHeight;camera.updateProjectionMatrix();renderer.setSize(innerWidth,innerHeight);renderer.setPixelRatio(Math.min(devicePixelRatio,1.75));});
frame();
}
