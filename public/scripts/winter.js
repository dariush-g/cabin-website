import * as THREE from '../vendor/three.module.js';

// Shared textures and instanced branches keep the forest detailed without
// creating a separate draw call for every needle or branch.
export function createWinterExterior(scene, canvasTexture) {
  let seed = 941;
  const random = () => { seed = (seed * 1664525 + 1013904223) >>> 0; return seed / 4294967296; };
  const bark = canvasTexture((c,w,h) => {
    c.fillStyle='#514d46'; c.fillRect(0,0,w,h);
    for(let i=0;i<2600;i++){
      const light=35+random()*55;
      c.strokeStyle=`rgb(${light+8},${light+4},${light})`;
      c.lineWidth=.5+random()*3;
      const x=random()*w,y=random()*h;
      c.beginPath();c.moveTo(x,y);c.lineTo(x+random()*5-2.5,y+8+random()*65);c.stroke();
    }
  },256,512);
  bark.wrapS=bark.wrapT=THREE.RepeatWrapping;bark.repeat.set(2,3);
  const trunkMaterial=new THREE.MeshStandardMaterial({map:bark,bumpMap:bark,bumpScale:.045,roughness:1});
  // Transparent fronds have individually drawn needles, irregular tips and
  // snow caught along the top of each twig, rather than solid cone surfaces.
  const needles=canvasTexture((c,w,h)=>{
    c.clearRect(0,0,w,h);c.lineCap='round';
    c.strokeStyle='#514a37';c.lineWidth=5;
    c.beginPath();c.moveTo(12,h*.52);c.quadraticCurveTo(w*.5,h*.5,w-10,h*.58);c.stroke();
    for(let i=0;i<29;i++){
      const x=15+i*(w-35)/29,t=i/29;
      const length=(1-t*.78)*(45+random()*28);
      for(const side of [-1,1]){
        const endX=x+length*.58,endY=h*.52+side*length;
        c.strokeStyle='#4a5140';c.lineWidth=1.7;
        c.beginPath();c.moveTo(x,h*.52);c.lineTo(endX,endY);c.stroke();
        for(let j=0;j<15;j++){
          const f=j/15,px=x+(endX-x)*f,py=h*.52+(endY-h*.52)*f;
          const len=6+random()*12;
          c.strokeStyle=['#293d32','#405749','#617563','#718473'][Math.floor(random()*4)];
          c.lineWidth=.8+random();
          for(const spread of [-1,1]){
            c.beginPath();c.moveTo(px,py);c.lineTo(px+len*.75,py+side*len*.4+spread*len*.55);c.stroke();
          }
        }
        if(random()>.16){
          c.strokeStyle=['#cbd6d9','#e5ecec','#f2f5f2'][i%3];c.lineWidth=3+random()*5;
          c.beginPath();c.moveTo(x+4,h*.52-3);c.lineTo(endX*.91+x*.09,endY-3);c.stroke();
        }
      }
    }
    c.strokeStyle='#e3e9e7';c.lineWidth=3;
    c.beginPath();c.moveTo(12,h*.52-3);c.quadraticCurveTo(w*.45,h*.49,w*.8,h*.55-3);c.stroke();
  },512,192);
  const foliageMaterial=new THREE.MeshStandardMaterial({map:needles,alphaTest:.32,side:THREE.DoubleSide,roughness:1,emissiveMap:needles,emissive:'#9aaebc',emissiveIntensity:.2});
  const treeCount=30, tiers=14, arms=7;
  const trunks=new THREE.InstancedMesh(new THREE.CylinderGeometry(.08,.2,1,10),trunkMaterial,treeCount);
  const foliage=new THREE.InstancedMesh(new THREE.PlaneGeometry(1,1),foliageMaterial,treeCount*tiers*arms*2);
  const twigMaterial=new THREE.MeshStandardMaterial({color:'#5b5c50',roughness:1});
  const branches=new THREE.InstancedMesh(new THREE.CylinderGeometry(.008,.025,1,5),twigMaterial,treeCount*tiers*arms);
  const transform=new THREE.Object3D(), direction=new THREE.Vector3(), up=new THREE.Vector3(0,1,0);
  let leafIndex=0, branchIndex=0;
  for(let tree=0;tree<treeCount;tree++){
    const x=tree<6?-10-random()*5:-17-random()*22;
    const z=tree<6?-9+tree*3.4:-24+random()*48;
    const height=5.5+random()*5,width=1.5+random()*.8,offset=random()*Math.PI*2;
    // Keep the tapered trunk below the upper needle clusters.
    transform.position.set(x,height*.45,z);transform.rotation.set(0,offset,0);transform.scale.set(1,height*.9,1);
    transform.updateMatrix();trunks.setMatrixAt(tree,transform.matrix);
    for(let tier=0;tier<tiers;tier++){
      const fraction=tier/tiers,y=.9+fraction*(height-.95)+(random()-.5)*.22;
      const radius=width*Math.pow(1-fraction,.8)*(.85+random()*.3);
      for(let arm=0;arm<arms;arm++){
        const angle=offset+arm*Math.PI*2/arms+tier*.61;
        const length=radius*(.72+random()*.35),dip=.12+length*.14;
        const dx=Math.cos(angle),dz=Math.sin(angle);
        direction.set(dx*length,-dip,dz*length);
        transform.position.set(x+dx*length/2,y-dip/2,z+dz*length/2);
        transform.quaternion.setFromUnitVectors(up,direction.clone().normalize());
        transform.scale.set(1,direction.length(),1);transform.updateMatrix();branches.setMatrixAt(branchIndex++,transform.matrix);
        for(const tilt of [-.42,.42]){
          transform.position.set(x+dx*length*.58,y-dip*.6,z+dz*length*.58);
          transform.rotation.set(0,-angle,0);transform.rotateX(-Math.PI/2+tilt+(random()-.5)*.2);transform.rotateY(-.12);
          transform.scale.set(length*1.35,.48+length*.42,1);transform.updateMatrix();foliage.setMatrixAt(leafIndex++,transform.matrix);
        }
      }
    }
  }
  trunks.receiveShadow=true;foliage.receiveShadow=true;
  // The daylight source is overcast, so exterior foliage needs no hard shadows.
  scene.add(trunks,branches,foliage);
  const snowGroundTexture=canvasTexture((c,w,h)=>{
    c.fillStyle='#d2dcdf';c.fillRect(0,0,w,h);
    for(let i=0;i<12000;i++){const v=180+random()*65;c.fillStyle=`rgba(${v},${v+3},${Math.min(v+8,255)},.18)`;c.fillRect(random()*w,random()*h,1+random()*3,1);}
  });
  snowGroundTexture.wrapS=snowGroundTexture.wrapT=THREE.RepeatWrapping;snowGroundTexture.repeat.set(12,16);
  const ground=new THREE.Mesh(new THREE.PlaneGeometry(55,70),new THREE.MeshStandardMaterial({map:snowGroundTexture,roughness:1,emissiveMap:snowGroundTexture,emissive:'#a5b7c5',emissiveIntensity:.35}));
  ground.rotation.x=-Math.PI/2;ground.position.set(-32,-.03,0);scene.add(ground);
  // Drifts break up the flat ground at the forest edge.
  const driftMaterial=new THREE.MeshStandardMaterial({color:'#dce5e7',roughness:1,emissive:'#a5b7c5',emissiveIntensity:.35});
  for(let i=0;i<16;i++){
    const drift=new THREE.Mesh(new THREE.SphereGeometry(1,20,12),driftMaterial);
    drift.position.set(-7-random()*22,-.12,-15+random()*30);drift.scale.set(1.2+random()*2,.2+random()*.25,1.5+random()*3);scene.add(drift);
  }
  const flake=canvasTexture((c,w,h)=>{
    const gradient=c.createRadialGradient(w/2,h/2,0,w/2,h/2,w/2);
    gradient.addColorStop(0,'rgba(255,255,255,1)');gradient.addColorStop(.23,'rgba(255,255,255,.95)');gradient.addColorStop(.58,'rgba(255,255,255,.45)');gradient.addColorStop(1,'rgba(255,255,255,0)');
    c.fillStyle=gradient;c.fillRect(0,0,w,h);
  },64,64);
  const layers=[{count:2400,near:-15,far:-5.3,width:27,height:11,size:.075,speed:1.2},{count:3800,near:-43,far:-14,width:62,height:17,size:.1,speed:.9},{count:450,near:-12,far:-5.25,width:22,height:9,size:.14,speed:1.65}].map(config=>{
    const positions=new Float32Array(config.count*3),phases=new Float32Array(config.count);
    for(let i=0;i<config.count;i++){
      positions[i*3]=config.near+random()*(config.far-config.near);
      positions[i*3+1]=random()*config.height;positions[i*3+2]=(random()-.5)*config.width;phases[i]=random()*Math.PI*2;
    }
    const geometry=new THREE.BufferGeometry();geometry.setAttribute('position',new THREE.BufferAttribute(positions,3));
    const mesh=new THREE.Points(geometry,new THREE.PointsMaterial({color:'#eff7ff',map:flake,size:config.size,transparent:true,opacity:.82,depthWrite:false,alphaTest:.015}));
    mesh.renderOrder=1;scene.add(mesh);
    return {...config,positions,phases,geometry};
  });
  let time=0;
  return {
    update(dt){
      time+=dt;const gust=1.7+Math.sin(time*.63)*.9+Math.sin(time*1.41)*.35;
      for(const layer of layers){
        const {positions,phases,count,speed,width,height,near,far}=layer;
        for(let i=0;i<count;i++){
          const j=i*3,flutter=Math.sin(time*1.8+phases[i]);
          positions[j]+=dt*(gust*.75+flutter*.3)*speed;
          positions[j+1]-=dt*(1.3+Math.cos(phases[i])*.4)*speed;
          positions[j+2]+=dt*(gust*1.8+flutter*.7)*speed;
          if(positions[j]>far)positions[j]=near;
          if(positions[j+1]<0)positions[j+1]+=height;
          if(positions[j+2]>width/2)positions[j+2]-=width;
        }
        layer.geometry.attributes.position.needsUpdate=true;
      }
    }
  };
}
